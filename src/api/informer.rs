use crate::api::resource::{
    ResourceList,
    WatchEvent,
    ApiResource,
};
use crate::client::APIClient;
use crate::{Result};

use serde::de::DeserializeOwned;
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

type WatchQueue<T, U> = VecDeque<WatchEvent<T, U>>;

/// A rust reinterpretation of go's Informer
///
/// This watches a `Resource<T, U>`, by:
/// - seeding the intial resourceVersion with a list call
/// - keeping track of resourceVersions after every poll
/// - recovering when resourceVersions get desynced
///
/// Caches WatchEvents internally
/// and exposes only `WatchEvents` when you call `.poll()`.
#[derive(Clone)]
pub struct Informer<T, U> where
  T: Clone, U: Clone
{
    events: Arc<RwLock<WatchQueue<T, U>>>,
    version: Arc<RwLock<String>>,
    client: APIClient,
    resource: ApiResource,
}

impl<T, U> Informer<T, U> where
    T: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned,
{
    /// Create a reflector with a kube client on a kube resource
    ///
    /// Initializes resourceVersion with a 1 limit list call
    pub fn new(client: APIClient, r: ApiResource) -> Result<Self> {
        info!("Creating Informer for {:?}", r);
        let initial = get_resource_version(&client, &r)?;
        Ok(Informer {
            client,
            resource: r,
            events: Arc::new(RwLock::new(VecDeque::new())),
            version: Arc::new(RwLock::new(initial)),
        })
    }

    /// Create a reflector with a kube client on a kube resource
    ///
    /// Initializes resourceVersion from a passed in value
    pub fn from_version(client: APIClient, r: ApiResource, v: String) -> Result<Self> {
        info!("Creating Informer for {:?}", r);
        Ok(Informer {
            client,
            resource: r,
            events: Arc::new(RwLock::new(VecDeque::new())),
            version: Arc::new(RwLock::new(v)),
        })
    }

    /// Run a single watch poll
    ///
    /// If this returns an error, it resets the resourceVersion.
    /// This is meant to be run continually and events are meant to be handled between.
    /// If handling all the events is too time consuming, you probably need a queue.
    pub fn poll(&self) -> Result<()> {
        trace!("Watching {:?}", self.resource);
        let oldver = self.version();
        match watch_for_resource_updates(&self.client, &self.resource, &oldver) {
            Ok((events, newver)) => {
                *self.version.write().unwrap() = newver;
                for e in events {
                    self.events.write().unwrap().push_back(e);
                }
            },
            Err(e) => {
                warn!("Poll error: {:?}", e);
                // If desynched due to mismatching resourceVersion, retry in a bit
                std::thread::sleep(std::time::Duration::from_secs(10));
                self.reset()?;
            }
        };
        Ok(())
    }

    /// Pop an event from the front of the WatchQueue
    pub fn pop(&self) -> Option<WatchEvent<T, U>> {
        self.events.write().unwrap().pop_front()
    }

    /// Reset the resourceVersion to current and clear the event queue
    pub fn reset(&self) -> Result<()> {
        // Fetch a new initial version:
        let initial = get_resource_version(&self.client, &self.resource)?;
        *self.version.write().unwrap() = initial;
        self.events.write().unwrap().clear();
        Ok(())
    }

    /// Return the current version
    pub fn version(&self) -> String {
        self.version.read().unwrap().clone()
    }
}

fn get_resource_version(client: &APIClient, rg: &ApiResource) -> Result<String>
{
    let req = rg.list_zero_resource_entries()?;

    // parse to void a ResourceList into void except for Metadata
    #[derive(Clone, Deserialize)]
    struct Discard {} // ffs
    let res = client.request::<ResourceList<Option<Discard>>>(req)?;

    let version = res.metadata.resourceVersion.unwrap_or_else(|| "0".into());
    debug!("Got fresh resourceVersion={} for {}", version, rg.resource);
    Ok( version )
}


fn watch_for_resource_updates<T, U>(client: &APIClient, rg: &ApiResource, ver: &str)
    -> Result<(Vec<WatchEvent<T, U>>, String)> where
  T: Clone + DeserializeOwned,
  U: Clone + DeserializeOwned,
{
    let req = rg.watch_resource_entries_after(ver)?;
    let events = client.request_events::<WatchEvent<T, U>>(req)?;

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
    debug!("Got {} {} events, resourceVersion={}", events.len(), rg.resource, newver);

    Ok((events, newver))
}
