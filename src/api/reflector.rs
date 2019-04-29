use crate::api::resource::{
    list_all_crd_entries,
    watch_crd_entries_after,
    ResourceList,
    Resource,
    WatchEvent,
    ApiResource,
    Named,
};
use log::{info, warn, debug, trace};
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use crate::client::APIClient;
use crate::{Result};

use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
    time::{Duration},
};

/// A rust rewrite of client-go's Reflector
///
/// This is meant to watch and cache a resource T, by:
/// - allowing polling in the correct kube api way
/// - recovering when resourceVersions get desynced
///
/// It exposes it's internal state readably through a getter.
/// As such, a Reflector can be shared with actix-web as application state.
#[derive(Clone)]
pub struct Reflector<T> where
  T: Debug + Clone + Named
{
    /// Application state can be read continuously with read
    ///
    /// Write access to this data is entirely encapsulated within poll + refresh
    /// Users are meant to start a thread to poll, and maybe ask for a refresh.
    /// Beyond that, use the read call as a local cache.
    data: Arc<RwLock<Cache<T>>>,

    /// Kubernetes API Client
    client: APIClient,

    /// Api Resource this Reflector is responsible for
    resource: ApiResource,
}

impl<T> Reflector<T> where
    T: Debug + Clone + Named + DeserializeOwned
{
    /// Create a reflector with a kube client on a kube resource
    ///
    /// Initializes with a full list of data from a large initial LIST call
    pub fn new(client: APIClient, r: ApiResource) -> Result<Self> {
        info!("Creating Reflector for {:?}", r);
        let current : Cache<T> = get_resource_entries(&client, &r)?;
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
        use std::thread;
        trace!("Watching {:?}", self.resource);
        let old = self.data.read().unwrap().clone();
        match watch_for_resource_updates(&self.client, &self.resource, old) {
            Ok(res) => {
                *self.data.write().unwrap() = res;
            },
            Err(_e) => {
                // If desynched due to mismatching resourceVersion, retry in a bit
                thread::sleep(Duration::from_secs(10));
                self.refresh()?; // propagate error if this failed..
            }
        }

        Ok(())
    }

    /// Read data for users of the reflector
    pub fn read(&self) -> Result<Cache<T>> {
        // unwrap for users because Poison errors are not great to deal with atm.
        // If a read fails, you've probably failed to parse the Resource into a T
        // this likely implies versioning issues between:
        // - your definition of T (in code used to instantiate Reflector)
        // - current applied kube state (used to parse into T)
        //
        // Very little that can be done in this case. Upgrade your app / resource.
        let data = self.data.read().unwrap().clone();
        Ok(data)
    }

    /// Refresh the full resource state with a LIST call
    ///
    /// Same as what is done in `State::new`.
    pub fn refresh(&self) -> Result<()> {
        debug!("Refreshing {:?}", self.resource);
        let current : Cache<T> = get_resource_entries(&self.client, &self.resource)?;
        *self.data.write().unwrap() = current;
        Ok(())
    }
}

/// Public Resource Map typically exposed by the Reflector
pub type ResourceMap<T> = BTreeMap<String, T>;

/// Cache state used by a Reflector
#[derive(Default, Clone)]
pub struct Cache<T> {
    pub data: ResourceMap<T>,
    /// Current resourceVersion used for bookkeeping
    version: String,
}


pub fn get_resource_entries<T>(client: &APIClient, rg: &ApiResource) -> Result<Cache<T>> where
  T: Debug + Clone + Named + DeserializeOwned
{
    let req = list_all_crd_entries(&rg)?;
    let res = client.request::<ResourceList<Resource<T>>>(req)?;
    let mut data = BTreeMap::new();
    let version = res.metadata.resourceVersion;
    info!("Got {} with {} elements at resourceVersion={}", res.kind, res.items.len(), version);

    for i in res.items {
        data.insert(i.spec.name(), i.spec);
    }
    let keys = data.keys().cloned().collect::<Vec<_>>().join(", ");
    debug!("Initialized with: {}", keys);
    Ok(Cache { data, version })
}

pub fn watch_for_resource_updates<T>(client: &APIClient, rg: &ApiResource, mut c: Cache<T>)
    -> Result<Cache<T>> where
  T: Debug + Clone + Named + DeserializeOwned
{
    let req = watch_crd_entries_after(&rg, &c.version)?;
    let res = client.request_events::<WatchEvent<Resource<T>>>(req)?;

    // NB: events appear ordered, so the last one IS the max
    // We could parse the resourceVersion as uint and take the MAX for safety
    // but the api docs say not to rely on the format of resourceVersion anyway..
    for ev in res {
        debug!("Got {:?}", ev);
        match ev {
            WatchEvent::Added(o) => {
                info!("Adding {} to {}", o.spec.name(), rg.resource);
                c.data.entry(o.spec.name().clone())
                    .or_insert_with(|| o.spec.clone());
                if o.metadata.resourceVersion != "" {
                  c.version = o.metadata.resourceVersion.clone();
                }
            },
            WatchEvent::Modified(o) => {
                info!("Modifying {} in {}", o.spec.name(), rg.resource);
                c.data.entry(o.spec.name().clone())
                    .and_modify(|e| *e = o.spec.clone());
                if o.metadata.resourceVersion != "" {
                  c.version = o.metadata.resourceVersion.clone();
                }
            },
            WatchEvent::Deleted(o) => {
                info!("Removing {} from {}", o.spec.name(), rg.resource);
                c.data.remove(&o.spec.name());
                if o.metadata.resourceVersion != "" {
                  c.version = o.metadata.resourceVersion.clone();
                }
            }
            WatchEvent::Error(e) => {
                warn!("Failed to watch {}: {:?}", rg.resource, e);
                bail!("Failed to watch {}: {:?} - {:?}", rg.resource, e.message, e.reason)
            }
        }
    }
    //debug!("Updated: {}", found.join(", "));
    Ok(c) // updated in place (taken ownership)
}
