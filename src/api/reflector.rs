use crate::api::{RawApi, Api, ListParams, ObjectMeta};
use crate::api::resource::{
    ObjectList,
    WatchEvent,
    KubeObject,
};
use serde::de::DeserializeOwned;

use crate::client::APIClient;
use crate::{Result, ErrorKind};

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{Duration},
};

/// Internal representation for Reflector
type Cache<K> = BTreeMap<ObjectId, K>;

/// A reflection of `Resource` state in kubernetes
///
/// This watches and caches a `Resource<K>` by:
/// - seeding the cache from a large initial list call
/// - keeping track of initial, and subsequent resourceVersions
/// - recovering when resourceVersions get desynced
///
/// It exposes it's internal state readably through a getter.
#[derive(Clone)]
pub struct Reflector<K> where
    K: Clone + DeserializeOwned,
{
    data: Arc<RwLock<Cache<K>>>,
    version: Arc<RwLock<String>>,
    client: APIClient,
    resource: RawApi,
    params: ListParams,
}

impl<K> Reflector<K> where
    K: Clone + DeserializeOwned,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn new(r: Api<K>) -> Self {
        Reflector {
            client: r.client,
            resource: r.api,
            params: ListParams::default(),
            data: Arc::new(RwLock::new(BTreeMap::new())),
            version: Arc::new(RwLock::new(0.to_string())),
        }
    }
}


impl<K> Reflector<K> where
    K: Clone + DeserializeOwned + KubeObject,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn raw(client: APIClient, r: RawApi) -> Self {
        Reflector {
            client,
            resource: r,
            params: ListParams::default(),
            data: Arc::new(RwLock::new(BTreeMap::new())),
            version: Arc::new(RwLock::new(0.to_string())),
        }
    }

    // builders for ListParams - TODO: defer to internal informer in future?
    // for now, copy paste of informer's methods.

    /// Configure the timeout for the list/watch call.
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// Defaults to 10s
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
        let (data, version) = self.get_full_resource_entries().await?;
        *self.data.write().unwrap() = data;
        *self.version.write().unwrap() = version;
        Ok(self)
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread. Spawn one.
    pub async fn poll(&self) -> Result<()> {
        trace!("Watching {:?}", self.resource);
        if let Err(_e) = self.single_watch().await {
            // If desynched due to mismatching resourceVersion, retry in a bit
            // TODO: async sleep!?
            //std::thread::sleep(Duration::from_secs(10));
            self.reset().await?; // propagate error if this failed..
        }

        Ok(())
    }


    /// Read data for users of the reflector
    ///
    /// TODO: deprecate in favour of a stream returning fn..
    pub fn read(&self) -> Result<Vec<K>> {
        // unwrap for users because Poison errors are not great to deal with atm.
        // If a read fails, you've probably failed to parse the Resource into a T
        // this likely implies versioning issues between:
        // - your definition of T (in code used to instantiate Reflector)
        // - current applied kube state (used to parse into T)
        //
        // Very little that can be done in this case. Upgrade your app / resource.
        let cache = self.data.read().unwrap();

        Ok(cache
            .values()
            .cloned()
            .collect::<Vec<K>>()
        )
    }

    /// Read a single entry by name
    ///
    /// Will read in the configured namsepace, or globally on non-namespaced reflectors.
    /// If you are using a non-namespaced resources with name clashes,
    /// Try `Reflector::get_within` instead.
    pub fn get(&self, name: &str) -> Result<Option<K>> {
        let id = ObjectId {
          name: name.into(),
          namespace: self.resource.namespace.clone()
        };
        Ok(self.data.read().unwrap().get(&id).map(Clone::clone))
    }

    /// Read a single entry by name within a specific namespace
    ///
    /// This is a more specific version of `Reflector::get`.
    /// This is only useful if your reflector is configured to poll across namsepaces.
    pub fn get_within(&self, name: &str, ns: &str) -> Result<Option<K>> {
        let id = ObjectId {
          name: name.into(),
          namespace: Some(ns.into())
        };
        Ok(self.data.read().unwrap().get(&id).map(Clone::clone))
    }

    /// Reset the state with a full LIST call
    ///
    /// Same as what is done in `State::new`.
    pub async fn reset(&self) -> Result<()> {
        trace!("Refreshing {:?}", self.resource);
        let (data, version) = self.get_full_resource_entries().await?;
        *self.data.write().unwrap() = data;
        *self.version.write().unwrap() = version;
        Ok(())
    }


    async fn get_full_resource_entries(&self) -> Result<(Cache<K>, String)> {
        let req = self.resource.list(&self.params)?;
        // NB: Object isn't general enough here
        let res = self.client.request::<ObjectList<K>>(req).await?;
        let mut data = BTreeMap::new();
        let version = res.metadata.resourceVersion.unwrap_or_else(|| "".into());

        trace!("Got {} {} at resourceVersion={:?}", res.items.len(), self.resource.resource, version);
        for i in res.items {
            // The non-generic parts we care about are spec + status
            data.insert(i.meta().into(), i);
        }
        let keys = data.keys().map(|key: &ObjectId| key.to_string()).collect::<Vec<_>>().join(", ");
        debug!("Initialized with: {}", keys);
        Ok((data, version))
    }

    // Watch helper
    async fn single_watch(&self) -> Result<()> {
        let rg = &self.resource;
        let oldver = { self.version.read().unwrap().clone() };
        let req = rg.watch(&self.params, &oldver)?;
        let res = self.client.request_events::<WatchEvent<K>>(req).await?;

        // Update in place:
        let mut data = self.data.write().unwrap();
        let mut ver = self.version.write().unwrap();

        // Follow docs conventions and store the last resourceVersion
        // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
        for ev in res {
            match ev {
                WatchEvent::Added(o) => {
                    debug!("Adding {} to {}", o.meta().name, rg.resource);
                    data.entry(o.meta().into())
                        .or_insert_with(|| o.clone());
                    if let Some(v) = &o.meta().resourceVersion {
                        *ver = v.to_string();
                    }
                },
                WatchEvent::Modified(o) => {
                    debug!("Modifying {} in {}", o.meta().name, rg.resource);
                    data.entry(o.meta().into())
                        .and_modify(|e| *e = o.clone());
                    if let Some(v) = &o.meta().resourceVersion {
                        *ver = v.to_string();
                    }
                },
                WatchEvent::Deleted(o) => {
                    debug!("Removing {} from {}", o.meta().name, rg.resource);
                    data.remove(&o.meta().into());
                    if let Some(v) = &o.meta().resourceVersion {
                         *ver = v.to_string();
                    }
                }
                WatchEvent::Error(e) => {
                    warn!("Failed to watch {}: {:?}", rg.resource, e);
                    Err(ErrorKind::Api(e))?
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
