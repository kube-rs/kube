use crate::{
    api::{Api, ListParams, Meta, WatchEvent},
    Error, Result,
};
use futures::{future::FutureExt, lock::Mutex, pin_mut, select, TryStreamExt};
use serde::de::DeserializeOwned;
use tokio::{
    signal::{self, ctrl_c},
    time::delay_for,
};

use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// A Reflector with a default MapCache
pub type Reflector<K> = GenericReflector<K, MapCache<K>>;

/// A reflection of state for a Kubernetes ['Api'] resource
///
/// This builds on top of the ['Informer'] by tracking the events received,
/// via ['Informer::poll']. This object will in fact use .poll() continuously,
/// and use the results to maintain an up to date state map.
///
/// It is prone to the same desync problems as an informer, but it will self-heal,
/// as best as possible - though this means that you might occasionally see a full
/// reset (boot equivalent) when network issues are encountered.
/// During a reset, the state is cleared and rebuilt in an atomic operation.
///
/// The internal state is exposed readably through a getter.
#[derive(Clone)]
pub struct GenericReflector<K, S>
where
    K: Clone + DeserializeOwned + Meta,
    S: Store<K> + Default,
{
    store: Arc<Mutex<S>>,
    version: Arc<Mutex<String>>,
    params: ListParams,
    api: Api<K>,
}

impl<K, S> GenericReflector<K, S>
where
    K: Clone + DeserializeOwned + Meta,
    S: Store<K> + Default,
{
    /// Create a reflector on an api resource
    pub fn new(api: Api<K>) -> Self {
        Reflector {
            api,
            params: ListParams::default(),
            version: Arc::new(Mutex::new(0.to_string())),
            store: Default::default(),
        }
    }

    /// Modify the default watch parameters for the underlying watch
    pub fn params(mut self, lp: ListParams) -> Self {
        self.params = lp;
        self
    }

    /// Start the reflectors self-driving polling
    pub async fn run(self) -> Result<()> {
        self.reset().await?;
        loop {
            // local development needs listening for ctrl_c
            let ctrlc_fut = ctrl_c().fuse();
            // kubernetes apps need to listen for SIGTERM (30s warning)
            use signal::unix::{signal, SignalKind}; // TODO: conditional compile
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            let sigterm_fut = sigterm.recv().fuse();

            // and reflector needs to poll continuously
            let poll_fut = self.poll().fuse();

            // Then pin then futures to the stack, and wait for any of them
            pin_mut!(ctrlc_fut, sigterm_fut, poll_fut);
            select! {
                ctrlc = ctrlc_fut => {
                    warn!("Intercepted ctrl_c signal");
                    return Ok(());
                },
                sigterm = sigterm_fut => {
                    warn!("Intercepted SIGTERM");
                    return Ok(());
                }
                poll = poll_fut => {
                    // Error handle if not ok, otherwise, we do another iteration
                    if let Err(e) = poll {
                        warn!("Poll error on {}: {}: {:?}", self.api.resource.kind, e, e);
                        // If desynched due to mismatching resourceVersion, retry in a bit
                        let dur = Duration::from_secs(10);
                        delay_for(dur).await;
                        self.reset().await?; // propagate error if this failed..
                    }
                }
            }
        }
    }

    /// A single poll call to modify the internal state
    async fn poll(&self) -> Result<()> {
        let kind = &self.api.resource.kind;
        let resource_version = self.version.lock().await.clone();
        trace!("Polling {} from resourceVersion={}", kind, resource_version);
        let stream = self.api.watch(&self.params, &resource_version).await?;
        pin_mut!(stream);

        // For every event, modify our state
        while let Some(ev) = stream.try_next().await? {
            // Informer-like version tracking:
            match &ev {
                WatchEvent::Added(o)
                | WatchEvent::Modified(o)
                | WatchEvent::Deleted(o)
                | WatchEvent::Bookmark(o) => {
                    // always store the last seen resourceVersion
                    if let Some(nv) = Meta::resource_ver(o) {
                        trace!("Updating reflector version for {} to {}", kind, nv);
                        *self.version.lock().await = nv.clone();
                    }
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", kind, e);
                    return Err(Error::Api(e.to_owned()));
                }
            }

            // Core Reflector logic
            let mut store = self.store.lock().await;
            match ev {
                WatchEvent::Added(o) => store.add(o),
                WatchEvent::Modified(o) => store.modify(o),
                WatchEvent::Deleted(o) => store.delete(o),
                _ => {}
            }
        }
        Ok(())
    }

    /// Reset the resource version and clear cache
    async fn reset(&self) -> Result<()> {
        trace!("Resetting {}", self.api.resource.kind);

        let (data, version) = self.get_full_resource_entries().await?;
        *self.version.lock().await = version;
        let mut store = self.store.lock().await;
        store.clear();
        for d in data {
            store.add(d);
        }
        Ok(())
    }

    /// Legacy helper for kubernetes < 1.16
    ///
    /// Needed to do an initial list operation because of https://github.com/clux/kube-rs/issues/219
    /// Soon, this goes away as we drop support for k8s < 1.16
    async fn get_full_resource_entries(&self) -> Result<(Vec<K>, String)> {
        let res = self.api.list(&self.params).await?;
        debug!("Initializing {}", K::KIND);
        let version = res.metadata.resource_version.unwrap_or_default();
        debug!(
            "Initialized {} with {} objects at {}",
            K::KIND,
            res.items.len(),
            version
        );
        Ok((res.items, version))
    }

    /// Read data for users of the reflector
    ///
    /// This is instant if you are reading and writing from the same context.
    pub async fn state(&self) -> Result<Vec<K>> {
        let store = self.store.lock().await;
        Ok(store.values())
    }

    /// Read a single entry by name
    ///
    /// Will read in the configured namsepace, or globally on non-namespaced reflectors.
    /// If you are using a non-namespaced resources with name clashes,
    /// Try [`Reflector::get_within`] instead.
    pub async fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: self.api.resource.namespace.clone(),
        };

        Ok(self.store.lock().await.get(&id).map(Clone::clone))
    }

    /// Read a single entry by name within a specific namespace
    ///
    /// This is a more specific version of [`Reflector::get`].
    /// This is only useful if your reflector is configured to poll across namsepaces.
    pub async fn get_within(&self, name: &str, ns: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: Some(ns.into()),
        };
        Ok(self.store.lock().await.get(&id).map(Clone::clone))
    }
}

/// ObjectId represents an object by name and namespace (if any)
///
/// This is an internal subset of ['k8s_openapi::api::core::v1::ObjectReference']
#[derive(Ord, PartialOrd, Hash, Eq, PartialEq, Clone)]
pub struct ObjectId {
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

/// A store that can be plugged into a Reflector
pub trait Store<K> {
    fn clear(&mut self);
    fn get(&self, id: &ObjectId) -> Option<&K>;
    fn add(&mut self, k: K);
    fn values(&self) -> Vec<K>;
    fn modify(&mut self, k: K);
    fn delete(&mut self, k: K);
}

/// Default Store for a Reflector
pub type MapCache<K> = BTreeMap<ObjectId, K>;

impl<K> Store<K> for MapCache<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    fn clear(&mut self) {
        self.clear()
    }

    fn get(&self, id: &ObjectId) -> Option<&K> {
        self.get(id)
    }

    fn values(&self) -> Vec<K> {
        self.values().cloned().collect()
    }

    fn add(&mut self, k: K) {
        debug!("Adding {} to {}", Meta::name(&k), K::KIND);
        self.entry(ObjectId::key_for(&k)).or_insert_with(|| k);
    }

    fn modify(&mut self, k: K) {
        debug!("Modifying {} in {}", Meta::name(&k), K::KIND);
        self.entry(ObjectId::key_for(&k)).and_modify(|e| *e = k);
    }

    fn delete(&mut self, k: K) {
        debug!("Removing {} from {}", Meta::name(&k), K::KIND);
        self.remove(&ObjectId::key_for(&k));
    }
}
