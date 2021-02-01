use crate::{
    api::{Api, ListParams, Meta, WatchEvent},
    Error, Result,
};
use futures::{future::FutureExt, lock::Mutex, pin_mut, select, TryStreamExt};
use serde::de::DeserializeOwned;
use tokio::{signal::ctrl_c, time::sleep};

#[cfg(not(target_family = "windows"))] use tokio::signal;

#[cfg(target_family = "windows")] use tokio::sync::mpsc::{channel, Receiver};

use std::{collections::BTreeMap, sync::Arc, time::Duration};

/// A reflection of state for a Kubernetes ['Api'] resource
///
/// This builds on top of the ['Informer'] by tracking the events received,
/// via ['Informer::poll']. This object will in fact use .poll() continuously,
/// and use the results to maintain an up to date state map.
///
/// It is prone to the same desync problems as an informer, but it will self-heal,
/// as best as possible - though this means that you might occasionally see a full
/// reset (boot equivalent) when network issues are encountered.
/// During a reset, the state is cleared before it is rebuilt.
///
/// The internal state is exposed readably through a getter.
#[derive(Clone)]
#[deprecated(note = "Replaced by kube_runtime::reflector", since = "0.38.0")]
pub struct Reflector<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    state: Arc<Mutex<State<K>>>,
    params: ListParams,
    api: Api<K>,
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    /// Create a reflector on an api resource
    pub fn new(api: Api<K>) -> Self {
        Reflector {
            api,
            params: ListParams::default(),
            state: Default::default(),
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
            #[cfg(not(target_family = "windows"))] use signal::unix::{signal, SignalKind};
            #[cfg(not(target_family = "windows"))]
            let mut sigterm = signal(SignalKind::terminate()).unwrap();
            #[cfg(not(target_family = "windows"))]
            let sigterm_fut = sigterm.recv().fuse();

            #[cfg(target_family = "windows")]
            let (_tx, mut rx): (_, Receiver<()>) = channel(1);
            #[cfg(target_family = "windows")]
            let sigterm_fut = rx.recv().fuse();

            // and reflector needs to poll continuously
            let poll_fut = self.poll().fuse();

            // Then pin then futures to the stack, and wait for any of them
            pin_mut!(ctrlc_fut, sigterm_fut, poll_fut);
            select! {
                _ctrlc = ctrlc_fut => {
                    info!("Received ctrl_c, exiting");
                    return Ok(());
                },
                _sigterm = sigterm_fut => {
                    info!("Received SIGTERM, exiting");
                    return Ok(());
                }
                poll = poll_fut => {
                    // Error handle if not ok, otherwise, we do another iteration
                    if let Err(e) = poll {
                        warn!("Poll error on {}: {}: {:?}", self.api.resource.kind, e, e);
                        // If desynched due to mismatching resourceVersion, retry in a bit
                        let dur = Duration::from_secs(10);
                        sleep(dur).await;
                        self.reset().await?; // propagate error if this failed..
                    }
                }
            }
        }
    }

    /// A single poll call to modify the internal state
    async fn poll(&self) -> Result<()> {
        let kind = &self.api.resource.kind;
        let resource_version = self.state.lock().await.version.clone();
        trace!("Polling {} from resourceVersion={}", kind, resource_version);
        let stream = self.api.watch(&self.params, &resource_version).await?;
        pin_mut!(stream);

        // For every event, modify our state
        while let Some(ev) = stream.try_next().await? {
            let mut state = self.state.lock().await;
            // Informer-like version tracking:
            match &ev {
                WatchEvent::Added(o) | WatchEvent::Modified(o) | WatchEvent::Deleted(o) => {
                    // always store the last seen resourceVersion
                    if let Some(nv) = Meta::resource_ver(o) {
                        trace!("Updating reflector version for {} to {}", kind, nv);
                        state.version = nv.clone();
                    }
                }
                WatchEvent::Bookmark(bm) => {
                    let rv = &bm.metadata.resource_version;
                    trace!("Updating reflector version for {} to {}", kind, rv);
                    state.version = rv.clone();
                }
                _ => {}
            }

            let data = &mut state.data;
            // Core Reflector logic
            match ev {
                WatchEvent::Added(o) => {
                    debug!("Adding {} to {}", Meta::name(&o), kind);
                    data.entry(ObjectId::key_for(&o)).or_insert_with(|| o.clone());
                }
                WatchEvent::Modified(o) => {
                    debug!("Modifying {} in {}", Meta::name(&o), kind);
                    data.entry(ObjectId::key_for(&o)).and_modify(|e| *e = o.clone());
                }
                WatchEvent::Deleted(o) => {
                    debug!("Removing {} from {}", Meta::name(&o), kind);
                    data.remove(&ObjectId::key_for(&o));
                }
                WatchEvent::Bookmark(bm) => {
                    debug!("Bookmarking {}", &bm.types.kind);
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", kind, e);
                    return Err(Error::Api(e));
                }
            }
        }
        Ok(())
    }

    /// Reset the state of the underlying informer and clear the cache
    pub async fn reset(&self) -> Result<()> {
        trace!("Resetting {}", self.api.resource.kind);
        // Simplified for k8s >= 1.16
        //*self.state.lock().await = Default::default();
        //self.informer.reset().await

        // For now:
        let (data, version) = self.get_full_resource_entries().await?;
        *self.state.lock().await = State { data, version };
        Ok(())
    }

    /// Legacy helper for kubernetes < 1.16
    ///
    /// Needed to do an initial list operation because of https://github.com/clux/kube-rs/issues/219
    /// Soon, this goes away as we drop support for k8s < 1.16
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

    /// Read data for users of the reflector
    ///
    /// This is instant if you are reading and writing from the same context.
    pub async fn state(&self) -> Result<Vec<K>> {
        let state = self.state.lock().await;
        Ok(state.data.values().cloned().collect::<Vec<K>>())
    }

    /// Read a single entry by name
    ///
    /// Will read in the configured namespace, or globally on non-namespaced reflectors.
    /// If you are using a non-namespaced resources with name clashes,
    /// Try [`Reflector::get_within`] instead.
    pub async fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: self.api.resource.namespace.clone(),
        };

        Ok(self.state.lock().await.data.get(&id).map(Clone::clone))
    }

    /// Read a single entry by name within a specific namespace
    ///
    /// This is a more specific version of [`Reflector::get`].
    /// This is only useful if your reflector is configured to poll across namespaces.
    /// TODO: remove once #194 is resolved
    pub async fn get_within(&self, name: &str, ns: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: Some(ns.into()),
        };
        Ok(self.state.lock().await.data.get(&id).map(Clone::clone))
    }
}

/// ObjectId represents an object by name and namespace (if any)
///
/// This is an internal subset of ['k8s_openapi::api::core::v1::ObjectReference']
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

/// Internal shared state of Reflector
///
/// Can remove this in k8s >= 1.16 once this uses Informer
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
/// Internal representation for Reflector
type Cache<K> = BTreeMap<ObjectId, K>;
