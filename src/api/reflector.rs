use crate::api::resource::{
    ResourceList,
    Resource,
    WatchEvent,
    ApiResource,
};
use serde::de::DeserializeOwned;

use crate::client::APIClient;
use crate::Result;

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{Duration},
};

/// A rust reinterpretation of of client-go's Reflector
///
/// This watches and caches a `Resource<T, U>` by:
/// - seeding the cache from a large initial list call
/// - keeping track of initial, and subsequent resourceVersions
/// - recovering when resourceVersions get desynced
///
/// It exposes it's internal state readably through a getter.
#[derive(Clone)]
pub struct Reflector<T, U> where
  T: Clone, U: Clone
{
    /// Application state can be read continuously with read
    ///
    /// Write access to this data is entirely encapsulated within poll + refresh
    /// Users are meant to start a thread to poll, and maybe ask for a refresh.
    /// Beyond that, use the read call as a local cache.
    data: Arc<RwLock<Cache<T, U>>>,

    /// Kubernetes API Client
    client: APIClient,

    /// Api Resource this Reflector is responsible for
    resource: ApiResource,
}

impl<T, U> Reflector<T, U> where
    T: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned,
{
    /// Create a reflector with a kube client on a kube resource
    ///
    /// Initializes with a full list of data from a large initial LIST call
    pub fn new(client: APIClient, r: ApiResource) -> Result<Self> {
        info!("Creating Reflector for {:?}", r);
        let current : Cache<T, U> = get_resource_entries(&client, &r)?;
        Ok(Reflector {
            client,
            resource: r,
            data: Arc::new(RwLock::new(current)),
        })
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread. Spawn one.
    pub fn poll(&self) -> Result<()> {
        trace!("Watching {:?}", self.resource);
        let old = self.data.read().unwrap().clone();
        match watch_for_resource_updates(&self.client, &self.resource, old) {
            Ok(res) => {
                *self.data.write().unwrap() = res;
            },
            Err(_e) => {
                // If desynched due to mismatching resourceVersion, retry in a bit
                std::thread::sleep(Duration::from_secs(10));
                self.refresh()?; // propagate error if this failed..
            }
        }

        Ok(())
    }

    /// Read data for users of the reflector
    pub fn read(&self) -> Result<ResourceMap<T, U>> {
        // unwrap for users because Poison errors are not great to deal with atm.
        // If a read fails, you've probably failed to parse the Resource into a T
        // this likely implies versioning issues between:
        // - your definition of T (in code used to instantiate Reflector)
        // - current applied kube state (used to parse into T)
        //
        // Very little that can be done in this case. Upgrade your app / resource.
        let data = self.data.read().unwrap().clone().data;
        Ok(data)
    }

    /// Refresh the full resource state with a LIST call
    ///
    /// Same as what is done in `State::new`.
    pub fn refresh(&self) -> Result<()> {
        debug!("Refreshing {:?}", self.resource);
        let current : Cache<T, U> = get_resource_entries(&self.client, &self.resource)?;
        *self.data.write().unwrap() = current;
        Ok(())
    }
}

/// Resource map exposed by the Reflector from its cache
pub type ResourceMap<T, U> = BTreeMap<String, Resource<T,U>>;

/// Resource map exposed by a ReflectorSpec (when there's no Status)
pub type ResourceSpecMap<T> = BTreeMap<String, Resource<T, Option<()>>>;
/// Resource map exposed by a ReflectorStatus (when there's no Spec)
pub type ResourceStatusMap<U> = BTreeMap<String, Resource<Option<()>, U>>;

/// Reflector around a Spec object only (blank Status)
pub type ReflectorSpec<T> = Reflector<T, Option<()>>;
/// Reflector around a Status object only (blank Spec)
pub type ReflectorStatus<U> = Reflector<Option<()>, U>;

/// Cache state used by a Reflector
#[derive(Default, Clone)]
struct Cache<T, U> where U: Clone, T: Clone {
    pub data: ResourceMap<T, U>,
    /// Current resourceVersion used for bookkeeping
    version: String,
}

fn get_resource_entries<T, U>(client: &APIClient, rg: &ApiResource) -> Result<Cache<T, U>> where
  T: Clone + DeserializeOwned,
  U: Clone + DeserializeOwned,
{
    let req = rg.list_all_resource_entries()?;
    // NB: Resource isn't general enough here
    let res = client.request::<ResourceList<Resource<T, U>>>(req)?;
    let mut data = BTreeMap::new();
    let version = res.metadata.resourceVersion.unwrap_or_else(|| "".into());
    debug!("Got {} {} at resourceVersion={:?}", res.items.len(), rg.resource, version);

    for i in res.items {
        // The non-generic parts we care about are spec + status
        data.insert(i.metadata.name.clone(), i);
    }
    let keys = data.keys().cloned().collect::<Vec<_>>().join(", ");
    debug!("Initialized with: {}", keys);
    Ok(Cache { data, version })
}

fn watch_for_resource_updates<T, U>(client: &APIClient, rg: &ApiResource, mut c: Cache<T, U>)
    -> Result< Cache<T, U> > where
  T: Clone + DeserializeOwned,
  U: Clone + DeserializeOwned,
{
    let req = rg.watch_resource_entries_after(&c.version)?;
    let res = client.request_events::<WatchEvent<Resource<T, U>>>(req)?;

    // Follow docs conventions and store the last resourceVersion
    // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
    for ev in res {
        match ev {
            WatchEvent::Added(o) => {
                info!("Adding {} to {}", o.metadata.name, rg.resource);
                c.data.entry(o.metadata.name.clone())
                    .or_insert_with(|| o.clone());
                if let Some(v) = o.metadata.resourceVersion {
                  c.version = v;
                }
            },
            WatchEvent::Modified(o) => {
                info!("Modifying {} in {}", o.metadata.name, rg.resource);
                c.data.entry(o.metadata.name.clone())
                    .and_modify(|e| *e = o.clone());
                if let Some(v) = o.metadata.resourceVersion {
                  c.version = v;
                }
            },
            WatchEvent::Deleted(o) => {
                info!("Removing {} from {}", o.metadata.name, rg.resource);
                c.data.remove(&o.metadata.name);
                if let Some(v) = o.metadata.resourceVersion {
                  c.version = v;
                }
            }
            WatchEvent::Error(e) => {
                warn!("Failed to watch {}: {:?}", rg.resource, e);
                bail!("Failed to watch {}: {:?} - {:?}", rg.resource, e.message, e.reason)
            }
        }
    }
    Ok(c) // updated in place (taken ownership)
}
