use crate::{
    api::{ListParams, Meta, ObjectList, Resource, WatchEvent},
    Client, Error, Result,
};
use futures::{lock::Mutex, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use tokio::time::delay_for;

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
    K: Clone + DeserializeOwned + Send + Meta,
{
    state: Arc<Mutex<State<K>>>,
    client: Client,
    resource: Resource,
    params: ListParams,
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + Meta + Send,
{
    /// Create a reflector with a kube client on a resource
    pub fn new(client: Client, lp: ListParams, r: Resource) -> Self {
        Reflector {
            client,
            resource: r,
            params: lp,
            state: Default::default(),
        }
    }

    /// Initializes with a full list of data from a large initial LIST call
    pub async fn init(self) -> Result<Self> {
        info!("Starting Reflector for {}", self.resource.kind);
        self.reset().await?;
        Ok(self)
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread/task. Spawn one.
    pub async fn poll(&self) -> Result<()> {
        trace!("Watching {}", self.resource.kind);
        if let Err(e) = self.single_watch().await {
            warn!("Poll error on {}: {}: {:?}", self.resource.kind, e, e);
            // If desynched due to mismatching resourceVersion, retry in a bit
            let dur = Duration::from_secs(10);
            delay_for(dur).await;
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
        use crate::api::resource::ResourceScope;
        let namespace = match &self.resource.scope {
            ResourceScope::Namespace(ns) => Some(ns.to_owned()),
            _ => None,
        };
        let id = ObjectId {
            name: name.into(),
            // TODO: impl From<Resource> for ObjectId
            namespace,
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
        trace!("Refreshing {}", self.resource.kind);
        let (data, version) = self.get_full_resource_entries().await?;
        *self.state.lock().await = State { data, version };
        Ok(())
    }

    async fn get_full_resource_entries(&self) -> Result<(Cache<K>, String)> {
        let req = self.resource.list(&self.params)?;
        // NB: Object isn't general enough here
        let res = self.client.request::<ObjectList<K>>(req).await?;
        let mut data = BTreeMap::new();
        let version = res.metadata.resource_version.unwrap_or_else(|| "".into());

        trace!(
            "Got {} {} at resourceVersion={:?}",
            res.items.len(),
            self.resource.kind,
            version
        );
        for i in res.items {
            // The non-generic parts we care about are spec + status
            data.insert(ObjectId::key_for(&i), i);
        }
        let keys = data
            .keys()
            .map(ObjectId::to_string)
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
                    let name = Meta::name(&o);
                    debug!("Adding {} to {}", name, rg.kind);
                    state
                        .data
                        .entry(ObjectId::key_for(&o))
                        .or_insert_with(|| o.clone());
                    if let Some(v) = Meta::resource_ver(&o) {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Modified(o) => {
                    let name = Meta::name(&o);
                    debug!("Modifying {} in {}", name, rg.kind);
                    state
                        .data
                        .entry(ObjectId::key_for(&o))
                        .and_modify(|e| *e = o.clone());
                    if let Some(v) = Meta::resource_ver(&o) {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Deleted(o) => {
                    let name = Meta::name(&o);
                    debug!("Removing {} from {}", name, rg.kind);
                    state.data.remove(&ObjectId::key_for(&o));
                    if let Some(v) = Meta::resource_ver(&o) {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", rg.kind, e);
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

impl ObjectId {
    fn key_for<K: Meta>(o: &K) -> Self {
        ObjectId {
            name: Meta::name(o),
            namespace: Meta::namespace(o),
        }
    }
}
