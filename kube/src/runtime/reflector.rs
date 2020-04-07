use crate::{
    api::{Api, ListParams, Meta, Resource, WatchEvent},
    runtime::Informer,
    Error, Result,
};
use futures::{lock::Mutex, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;

use std::{collections::BTreeMap, sync::Arc};

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
pub struct Reflector<K>
where
    K: Clone + DeserializeOwned + Send + Meta,
{
    state: Arc<Mutex<State<K>>>,
    informer: Informer<K>,
    resource: Resource,
}

impl<K> Reflector<K>
where
    K: Clone + DeserializeOwned + Meta + Send + Sync,
{
    /// Create a reflector on an api resource
    pub fn new(api: Api<K>) -> Self {
        Reflector {
            resource: api.resource.clone(),
            informer: Informer::new(api),
            state: Default::default(),
        }
    }

    /// Modify the default watch parameters for the underlying watch
    pub fn params(mut self, lp: ListParams) -> Self {
        self.informer = self.informer.params(lp);
        self
    }

    /// Start the reflectors self-driving polling
    pub async fn run(self) -> Result<()> {
        use futures::{future::FutureExt, pin_mut, select};
        use tokio::signal;
        loop {
            let signal_fut = signal::ctrl_c().fuse(); // TODO: SIGTERM
            let stream_fut = self.poll().fuse();
            pin_mut!(signal_fut, stream_fut);
            select! {
                sig = signal_fut => {
                    warn!("Intercepted ctrl_c signal");
                    return Ok(());
                },
                stream = stream_fut => {
                    if let Err(e) = stream {
                        error!("Kube state failed to recover: {}", e);
                        return Err(e);
                    }
                },
                complete => continue, // another poll
                //default => panic!(), // never runs - futures runs first, then complete
            }
        }
    }

    /// A single poll call to modify the internal state
    async fn poll(&self) -> Result<()> {
        let kind = &self.resource.kind;
        trace!("Polling {}", kind);
        let mut stream = self.informer.poll().await?.boxed();

        // For every event, modify our state
        while let Some(ev) = stream.try_next().await? {
            let mut state = self.state.lock().await;
            match ev {
                WatchEvent::Added(o) => {
                    debug!("Adding {} to {}", Meta::name(&o), kind);
                    state.entry(ObjectId::key_for(&o)).or_insert_with(|| o.clone());
                }
                WatchEvent::Modified(o) => {
                    debug!("Modifying {} in {}", Meta::name(&o), kind);
                    state.entry(ObjectId::key_for(&o)).and_modify(|e| *e = o.clone());
                }
                WatchEvent::Deleted(o) => {
                    debug!("Removing {} from {}", Meta::name(&o), kind);
                    state.remove(&ObjectId::key_for(&o));
                }
                WatchEvent::Bookmark(o) => {
                    debug!("Bookmarking {} from {}", Meta::name(&o), kind);
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", kind, e);
                    return Err(Error::Api(e));
                }
            }
        }
        Ok(())
    }

    /// Read data for users of the reflector
    ///
    /// This is instant if you are reading and writing from the same context.
    pub async fn state(&self) -> Result<Vec<K>> {
        let state = self.state.lock().await;
        Ok(state.values().cloned().collect::<Vec<K>>())
    }

    /// Read a single entry by name
    ///
    /// Will read in the configured namsepace, or globally on non-namespaced reflectors.
    /// If you are using a non-namespaced resources with name clashes,
    /// Try [`Reflector::get_within`] instead.
    pub async fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
            name: name.into(),
            namespace: self.resource.namespace.clone(),
        };

        Ok(self.state.lock().await.get(&id).map(Clone::clone))
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
        Ok(self.state.lock().await.get(&id).map(Clone::clone))
    }

    /// Reset the state of the underlying informer and clear the cache
    pub async fn reset(&self) {
        trace!("Resetting {}", self.resource.kind);
        *self.state.lock().await = Default::default();
        self.informer.reset().await
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

/// Internal representation for Reflector
type State<K> = BTreeMap<ObjectId, K>;
