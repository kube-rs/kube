use crate::api::resource::{
    watch_resource_entries_after,
    list_zero_resource_entries,
    Resource,
    ResourceList,
    WatchEvent,
    ApiResource,
};
use crate::client::APIClient;
use crate::{Result};

use serde::de::DeserializeOwned;

use std::{
    marker::PhantomData,
    sync::{Arc, RwLock},
};

/// A rust reinterpretation of go's Informer
///
/// This is meant to watch and cache a resource T, by:
/// - allowing polling in the correct kube api way
/// - recovering when resourceVersions get desynced
///
/// It exposes it's internal state readably through a getter.
/// As such, a Informer can be shared with actix-web as application state.
#[derive(Clone)]
pub struct Informer<T, U> where
  T: Clone, U: Clone
{
    /// Current resourceVersion used for bookkeeping
    version: Arc<RwLock<String>>,

    /// Kubernetes API Client
    client: APIClient,

    /// Api Resource this Informer is responsible for
    resource: ApiResource,

    // Informers never actually store any of T or U, they just pass them on.
    p1: PhantomData<T>,
    p2: PhantomData<U>,
}

impl<T, U> Informer<T, U> where
    T: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned,
{
    /// Create a reflector with a kube client on a kube resource
    ///
    /// Initializes with blank version string to get all events
    pub fn new(client: APIClient, r: ApiResource) -> Result<Self> {
        info!("Creating Informer for {:?}", r);
        let initial = get_resource_version(&client, &r)?;
        Ok(Informer {
            client,
            resource: r,
            version: Arc::new(RwLock::new(initial)),
            p1: PhantomData, p2: PhantomData,
        })
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it tries a full refresh.
    /// This is meant to be run continually in a thread. Spawn one.
    pub fn poll(&self) -> Result<WatchEvents<T, U>> {
        trace!("Watching {:?}", self.resource);
        let oldver = { self.version.read().unwrap().clone() }; // avoid holding lock
        let evs = match watch_for_resource_updates(&self.client, &self.resource, &oldver) {
            Ok((events, newver)) => {
                *self.version.write().unwrap() = newver;
                events
            },
            Err(e) => {
                warn!("Poll error: {:?}", e);
                *self.version.write().unwrap() = "0".into(); // reset ver if failed
                vec![]
            }
        };
        Ok(evs)
    }
}

/// Convenience alias around WatchEvents
pub type WatchEvents<T, U> = Vec<WatchEvent<Resource<T, U>>>;

/// Convenience aliases when only grabbing one of the fields
pub type InformerSpec<T> = Informer<T, Option<()>>;
pub type InformerStatus<U> = Informer<Option<()>, U>;

fn get_resource_version(client: &APIClient, rg: &ApiResource) -> Result<String>
{
    let req = list_zero_resource_entries(&rg)?;

    // parse to void a ResourceList into void except for Metadata
    #[derive(Clone, Deserialize)]
    struct Discard {} // ffs
    let res = client.request::<ResourceList<Option<Discard>>>(req)?;

    let version = res.metadata.resourceVersion.unwrap_or_else(|| "0".into());
    debug!("Got fresh resourceVersion={} for {}", version, rg.resource);
    Ok( version )
}


fn watch_for_resource_updates<T, U>(client: &APIClient, rg: &ApiResource, ver: &str)
    -> Result<(Vec<WatchEvent<Resource<T, U>>>, String)> where
  T: Clone + DeserializeOwned,
  U: Clone + DeserializeOwned,
{
    let req = watch_resource_entries_after(&rg, ver)?;
    let events = client.request_events::<WatchEvent<Resource<T, U>>>(req)?;
    debug!("Got {} events for {}", events.len(), rg.resource);

    // find last resourceVer and pass that on to avoid fetching duplicate events:
    let newver = events.iter().filter_map(|e| {
        match e {
            WatchEvent::Added(o) => o.metadata.resourceVersion.clone(),
            WatchEvent::Modified(o) => o.metadata.resourceVersion.clone(),
            WatchEvent::Deleted(o) => o.metadata.resourceVersion.clone(),
            _ => None
        }
    }).last().unwrap_or_else(|| ver.into());
    debug!("New resourceVersion for {} is {}", rg.resource, newver);

    Ok((events, newver))
}
