use crate::{
    api::{Api, ListParams, Meta, WatchEvent},
    Error, Result,
};
use futures::{lock::Mutex, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use tokio::time::delay_for;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// A reflection of state for a Kubernetes ['Api'] resource
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
    api: Api<K>,
    params: ListParams,
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + Meta + Send,
{
    /// Create a reflector on an api resource with a set of parameters
    pub fn new(api: Api<K>, lp: ListParams) -> Self {
        Reflector {
            api,
            params: lp,
            state: Default::default(),
        }
    }

    /// Initializes with a full list of data from a large initial LIST call
    pub async fn init(self) -> Result<Self> {
        info!("Starting Reflector for {}", self.api.resource.kind);
        self.reset().await?;
        Ok(self)
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread/task. Spawn one.
    pub async fn poll(&self) -> Result<()> {
        trace!("Watching {}", self.api.resource.kind);
        if let Err(e) = self.single_watch().await {
            warn!("Poll error on {}: {}: {:?}", self.api.resource.kind, e, e);
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
    /// Try [`Reflector::get_within`] instead.
    pub fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: self.api.resource.namespace.clone(),
        };

        futures::executor::block_on(async { Ok(self.state.lock().await.data.get(&id).map(Clone::clone)) })
    }

    /// Read a single entry by name within a specific namespace
    ///
    /// This is a more specific version of [`Reflector::get`].
    /// This is only useful if your reflector is configured to poll across namsepaces.
    pub fn get_within(&self, name: &str, ns: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: Some(ns.into()),
        };
        futures::executor::block_on(async { Ok(self.state.lock().await.data.get(&id).map(Clone::clone)) })
    }

    /// Reset the state with a full LIST call
    pub async fn reset(&self) -> Result<()> {
        trace!("Refreshing {}", self.api.resource.kind);
        let (data, version) = self.get_full_resource_entries().await?;
        *self.state.lock().await = State { data, version };
        Ok(())
    }

    async fn get_full_resource_entries(&self) -> Result<(Cache<K>, String)> {
        let res = self.api.list(&self.params).await?;
        let version = res.metadata.resource_version.unwrap_or_default();
        trace!(
            "Got {} {} at resourceVersion={:?}",
            res.items.len(),
            self.api.resource.kind,
            version
        );
        let mut data = BTreeMap::new();
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
        let rg = &self.api.resource;
        let oldver = self.state.lock().await.version.clone();
        let mut events = self.api.watch(&self.params, &oldver).await?.boxed();

        // Follow docs conventions and store the last resourceVersion
        // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
        while let Some(ev) = events.try_next().await? {
            // Update in place:
            let mut state = self.state.lock().await;
            match ev {
                WatchEvent::Added(o) => {
                    debug!("Adding {} to {}", Meta::name(&o), rg.kind);
                    state
                        .data
                        .entry(ObjectId::key_for(&o))
                        .or_insert_with(|| o.clone());
                    if let Some(v) = Meta::resource_ver(&o) {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Modified(o) => {
                    debug!("Modifying {} in {}", Meta::name(&o), rg.kind);
                    state
                        .data
                        .entry(ObjectId::key_for(&o))
                        .and_modify(|e| *e = o.clone());
                    if let Some(v) = Meta::resource_ver(&o) {
                        state.version = v.to_string();
                    }
                }
                WatchEvent::Deleted(o) => {
                    debug!("Removing {} from {}", Meta::name(&o), rg.kind);
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
