use crate::api::resource::{
    ResourceList,
    WatchEvent,
    ApiResource,
    QueryParams,
};
use crate::client::APIClient;
use crate::{Result};

use serde::de::DeserializeOwned;
use std::{
    collections::VecDeque,
    sync::{Arc, RwLock},
};

type WatchQueue<T, U> = VecDeque<WatchEvent<T, U>>;

/// An event informer for a `Resource`
///
/// This watches a `Resource<T, U>`, by:
/// - seeding the intial resourceVersion with a list call (optional)
/// - keeping track of resourceVersions after every poll
/// - recovering when resourceVersions get desynced
///
/// It caches WatchEvent<T, U> internally in a queue when polling.
/// A user should drain this queue periodically.
#[derive(Clone)]
pub struct Informer<T, U> where
  T: Clone, U: Clone
{
    events: Arc<RwLock<WatchQueue<T, U>>>,
    version: Arc<RwLock<String>>,
    client: APIClient,
    resource: ApiResource,
    params: QueryParams,
}

impl<T, U> Informer<T, U> where
    T: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn new(client: APIClient, r: ApiResource) -> Self {
        Informer {
            client,
            resource: r,
            params: QueryParams::default(),
            events: Arc::new(RwLock::new(VecDeque::new())),
            version: Arc::new(RwLock::new(0.to_string())),
        }
    }

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


    /// Initialize without a prior version
    ///
    /// Will seed resourceVersion with a 1 limit list call to the resource
    pub fn init(self) -> Result<Self> {
        let initial = self.get_resource_version()?;
        info!("Starting Informer for {:?}", self.resource);
        *self.version.write().unwrap() = initial;
        Ok(self)
    }

    /// Initialize from a prior version
    pub fn init_from(self, v: String) -> Self {
        info!("Recreating Informer for {:?} at {}", self.resource, v);
        *self.version.write().unwrap() = v;
        self
    }


    /// Run a single watch poll
    ///
    /// If this returns an error, it resets the resourceVersion.
    /// This is meant to be run continually and events are meant to be handled between.
    /// If handling all the events is too time consuming, you probably need a queue.
    pub fn poll(&self) -> Result<()> {
        trace!("Watching {:?}", self.resource);
        match self.single_watch() {
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
        let initial = self.get_resource_version()?;
        *self.version.write().unwrap() = initial;
        self.events.write().unwrap().clear();
        Ok(())
    }

    /// Return the current version
    pub fn version(&self) -> String {
        self.version.read().unwrap().clone()
    }


    /// Init helper
    fn get_resource_version(&self) -> Result<String> {
        let req = self.resource.list_zero_resource_entries(&self.params)?;

        // parse to void a ResourceList into void except for Metadata
        #[derive(Clone, Deserialize)]
        struct Discard {} // ffs
        let res = self.client.request::<ResourceList<Option<Discard>>>(req)?;

        let version = res.metadata.resourceVersion.unwrap_or_else(|| "0".into());
        debug!("Got fresh resourceVersion={} for {}", version, self.resource.resource);
        Ok( version )
    }

    /// Watch helper
    fn single_watch(&self) -> Result<(Vec<WatchEvent<T, U>>, String)> {
        let oldver = self.version();
        let req = self.resource.watch_resource_entries_after(&self.params, &oldver)?;
        let events = self.client.request_events::<WatchEvent<T, U>>(req)?;

        // Follow docs conventions and store the last resourceVersion
        // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
        let newver = events.iter().filter_map(|e| {
            match e {
                WatchEvent::Added(o) => o.metadata.resourceVersion.clone(),
                WatchEvent::Modified(o) => o.metadata.resourceVersion.clone(),
                WatchEvent::Deleted(o) => o.metadata.resourceVersion.clone(),
                _ => None
            }
        }).last().unwrap_or_else(|| oldver.into());
        debug!("Got {} {} events, resourceVersion={}", events.len(), self.resource.resource, newver);

        Ok((events, newver))
    }
}
