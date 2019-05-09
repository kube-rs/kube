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
/// This watches a `Resource<T, U>`, by:
/// - seeding the intial resourceVersion with a list call
/// - keeping track of resourceVersions after every poll
/// - recovering when resourceVersions get desynced
///
/// It contains no internal state except the `resourceVersion`,
/// and exposes only `WatchEvents` when you call `.poll()`.
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
    /// If this returns an error, it resets the resourceVersion.
    /// This is meant to be run continually and events are meant to be handled between.
    /// If handling all the events is too time consuming, you probably need a queue.
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
                // If desynched due to mismatching resourceVersion, retry in a bit
                std::thread::sleep(std::time::Duration::from_secs(10));
                // Fetch a new initial version:
                let initial = get_resource_version(&self.client, &self.resource)?;
                *self.version.write().unwrap() = initial;
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

    // Follow docs conventions and store the last resourceVersion
    // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
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
