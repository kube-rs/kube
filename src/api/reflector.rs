use crate::{
    api::{
        resource::{KubeObject, ObjectList, WatchEvent},
        Api, ListParams, ObjectMeta, RawApi,
    },
    client::APIClient,
    Error, Result,
};
use futures::{lock::Mutex, TryStreamExt, StreamExt};
use futures_timer::Delay;
use serde::de::DeserializeOwned;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// Internal representation for Reflector
type Cache<K> = BTreeMap<ObjectId, K>;

/// Internal shared state of Reflector
struct State<K> {
    data: Cache<K>,
    version: String,
}

impl<K> Default for State<K> {
    fn default() -> Self {
        State {
            data: Default::default(),
            version: 0.to_string(),
        }
    }
}

/// A reflection of `Resource` state in kubernetes
///
/// This watches and caches a `Resource<K>` by:
/// - seeding the cache from a large initial list call
/// - keeping track of initial, and subsequent resourceVersions
/// - recovering when resourceVersions get desynced
///
/// It exposes it's internal state readably through a getter.
#[derive(Clone)]
pub struct Reflector<K>
where
    K: Clone + DeserializeOwned + Send,
{
    state: Arc<Mutex<State<K>>>,
    client: APIClient,
    resource: RawApi,
    params: ListParams,
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + Send,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn new(r: Api<K>) -> Self {
        Reflector {
            client: r.client,
            resource: r.api,
            params: ListParams::default(),
            state: Default::default(),
        }
    }
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + KubeObject + Send,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn raw(client: APIClient, r: RawApi) -> Self {
        Reflector {
            client,
            resource: r,
            params: ListParams::default(),
            state: Default::default(),
        }
    }

    // builders for ListParams - TODO: defer to internal informer in future?
    // for now, copy paste of informer's methods.

    /// Configure the timeout for the list/watch call.
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// Defaults to 300s
    pub fn timeout(mut self, timeout_secs: u32) -> Self {
        self.params.timeout = Some(timeout_secs);
        self
    }

    /// Configure the selector to restrict the list of returned objects by their fields.
    ///
    /// Defaults to everything.
    /// Supports '=', '==', and '!=', and can comma separate: key1=value1,key2=value2
    /// The server only supports a limited number of field queries per type.
    pub fn fields(mut self, field_selector: &str) -> Self {
        self.params.field_selector = Some(field_selector.to_string());
        self
    }

    /// Configure the selector to restrict the list of returned objects by their labels.
    ///
    /// Defaults to everything.
    /// Supports '=', '==', and '!=', and can comma separate: key1=value1,key2=value2
    pub fn labels(mut self, label_selector: &str) -> Self {
        self.params.label_selector = Some(label_selector.to_string());
        self
    }

    /// If called, partially initialized resources are included in watch/list responses.
    pub fn include_uninitialized(mut self) -> Self {
        self.params.include_uninitialized = true;
        self
    }

    // finalizers:

    /// Initializes with a full list of data from a large initial LIST call
    pub async fn init(self) -> Result<Self> {
        info!("Starting Reflector for {:?}", self.resource);
        self.reset().await?;
        Ok(self)
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread/task. Spawn one.
    pub async fn poll(&self) -> Result<()> {
        trace!("Watching {:?}", self.resource);
        if let Err(_e) = self.single_watch().await {
            // If desynched due to mismatching resourceVersion, retry in a bit
            let dur = Duration::from_secs(10);
            Delay::new(dur).await;
            self.reset().await?; // propagate error if this failed..
        }

        Ok(())
    }

    /// Read data for users of the reflector
    ///
    /// This is instant if you are reading and writing from the same context.
    pub async fn state(&self) -> Result<Vec<K>> {
        let state = self.state.lock().await;
        Ok(state.data.values().cloned().collect::<Vec<K>>())
    }

    /// Read a single entry by name
    ///
    /// Will read in the configured namsepace, or globally on non-namespaced reflectors.
    /// If you are using a non-namespaced resources with name clashes,
    /// Try `Reflector::get_within` instead.
    pub fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: self.resource.namespace.clone(),
        };

        futures::executor::block_on(async { Ok(self.state.lock().await.data.get(&id).map(Clone::clone)) })
    }

    /// Read a single entry by name within a specific namespace
    ///
    /// This is a more specific version of `Reflector::get`.
    /// This is only useful if your reflector is configured to poll across namsepaces.
    pub fn get_within(&self, name: &str, ns: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: Some(ns.into()),
        };
        futures::executor::block_on(async { Ok(self.state.lock().await.data.get(&id).map(Clone::clone)) })
    }

    /// Reset the state with a full LIST call
    ///
    /// Same as what is done in `State::new`.
    pub async fn reset(&self) -> Result<()> {
        trace!("Refreshing {:?}", self.resource);
        let (data, version) = self.get_full_resource_entries().await?;
        *self.state.lock().await = State { data, version };
        Ok(())
    }

    async fn get_full_resource_entries(&self) -> Result<(Cache<K>, String)> {
        let req = self.resource.list(&self.params)?;
        // NB: Object isn't general enough here
        let res = self.client.request::<ObjectList<K>>(req).await?;
        let mut data = BTreeMap::new();
        let version = res.metadata.resourceVersion.unwrap_or_else(|| "".into());

        trace!(
            "Got {} {} at resourceVersion={:?}",
            res.items.len(),
            self.resource.resource,
            version
        );
        for i in res.items {
            // The non-generic parts we care about are spec + status
            data.insert(i.meta().into(), i);
        }
        let keys = data
            .keys()
            .map(|key: &ObjectId| key.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        debug!("Initialized with: [{}]", keys);
        Ok((data, version))
    }

    // Watch helper
    async fn single_watch(&self) -> Result<()> {
        let rg = &self.resource;
        let oldver = self.state.lock().await.version.clone();
        let req = rg.watch(&self.params, &oldver)?;
        let mut events = self.client.request_events::<WatchEvent<K>>(req).await?.boxed();

        // Follow docs conventions and store the last resourceVersion
        // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
        while let Some(ev) = events.try_next().await? {
            // Update in place:
            let mut state = self.state.lock().await;
            match ev {
                WatchEvent::Added(o) => {
                    debug!("Adding {} to {}", o.meta().name, rg.resource);
                    state.data.entry(o.meta().into()).or_insert_with(|| o.clone());
                    if let Some(v) = &o.meta().resourceVersion {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Modified(o) => {
                    debug!("Modifying {} in {}", o.meta().name, rg.resource);
                    state.data.entry(o.meta().into()).and_modify(|e| *e = o.clone());
                    if let Some(v) = &o.meta().resourceVersion {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Deleted(o) => {
                    debug!("Removing {} from {}", o.meta().name, rg.resource);
                    state.data.remove(&o.meta().into());
                    if let Some(v) = &o.meta().resourceVersion {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", rg.resource, e);
                    return Err(Error::Api(e));
                }
            }
        }
        Ok(())
    }
}

/// ObjectId represents an object by name and namespace (if any)
#[derive(Ord, PartialOrd, Hash, Eq, PartialEq, Clone)]
struct ObjectId {
    name: String,
    namespace: Option<String>,
}

impl ToString for ObjectId {
    fn to_string(&self) -> String {
        match &self.namespace {
            Some(ns) => format!("{} [{}]", self.name, ns),
            None => self.name.clone(),
        }
    }
}

impl From<&ObjectMeta> for ObjectId {
    fn from(object_meta: &ObjectMeta) -> Self {
        ObjectId {
            name: object_meta.name.clone(),
            namespace: object_meta.namespace.clone(),
        }
    }
}
