use crate::api::resource::{KubeObject, ObjectList, WatchEvent};
use crate::api::{Api, ListParams, RawApi, Void};
use crate::client::APIClient;
use crate::Result;

use futures::{Stream, StreamExt};
use futures_timer::Delay;
use serde::de::DeserializeOwned;
use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

/// An event informer for a `Resource`
///
/// This watches a `Resource<K>`, by:
/// - seeding the intial resourceVersion with a list call (optional)
/// - keeping track of resourceVersions after every poll
/// - recovering when resourceVersions get desynced
///
/// It caches WatchEvent<K> internally in a queue when polling.
/// A user should drain this queue periodically.
#[derive(Clone)]
pub struct Informer<K>
where
    K: Clone + DeserializeOwned + KubeObject,
{
    version: Arc<RwLock<String>>,
    client: APIClient,
    resource: RawApi,
    params: ListParams,
    needs_resync: Arc<RwLock<bool>>,
    _object_type: std::marker::PhantomData<K>,
}

impl<K> Informer<K>
where
    K: Clone + DeserializeOwned + KubeObject,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn new(r: Api<K>) -> Self {
        Informer {
            client: r.client,
            resource: r.api,
            params: ListParams::default(),
            version: Arc::new(RwLock::new(0.to_string())),
            needs_resync: Arc::new(RwLock::new(false)),
            _object_type: std::marker::PhantomData,
        }
    }
}

impl<K> Informer<K>
where
    K: Clone + DeserializeOwned + KubeObject,
{
    /// Create a reflector with a kube client on a kube resource
    pub fn raw(client: APIClient, r: RawApi) -> Self {
        Informer {
            client,
            resource: r,
            params: ListParams::default(),
            version: Arc::new(RwLock::new(0.to_string())),
            needs_resync: Arc::new(RwLock::new(false)),
            _object_type: std::marker::PhantomData,
        }
    }

    // builders for GetParams

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

    /// Initialize without a prior version
    ///
    /// Will seed resourceVersion with a 1 limit list call to the resource
    pub async fn init(self) -> Result<Self> {
        let initial = self.get_resource_version().await?;
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
    /// poll returns a Stream so events can be handled asynchronously
    pub async fn poll(&self) -> Result<impl Stream<Item = Result<WatchEvent<K>>>> {
        trace!("Watching {:?}", self.resource);

        // First check if we need to resync, if so reset our resource version
        // and wait a bit before proceeding.
        // We take a read only lock here as most of the time it will not be necessary
        // to take an exclusive lock.
        if *self.needs_resync.read().unwrap() {
            // If desynched due to mismatching resourceVersion, retry in a bit
            let dur = Duration::from_secs(10);
            Delay::new(dur).await;
            self.reset().await?;
            *self.needs_resync.write().unwrap() = false;
        }

        // Create our watch request
        let req = self.resource.watch(&self.params, &self.version())?;

        // Clone our version so we can move it into the Stream handling
        // and avoid a 'static lifetime on self
        let version = self.version.clone();

        // Clone our resync flag similarly
        let needs_resync = self.needs_resync.clone();

        // Attempt to fetch our stream
        let stream = self.client.request_events::<WatchEvent<K>>(req).await;

        match stream {
            Ok(events) => {
                // Add a map stage to the stream which will update our version
                // based on each incoming event
                Ok(events.map(move |event| {
                    // Check if we need to update our version based on the incoming events
                    let new_version = match &event {
                        Ok(WatchEvent::Added(o))
                        | Ok(WatchEvent::Modified(o))
                        | Ok(WatchEvent::Deleted(o)) => o.meta().resourceVersion.clone(),
                        _ => None,
                    };

                    // If we hit an error, mark that we need to resync on the next call
                    if let Err(e) = &event {
                        warn!("Poll error: {:?}", e);
                        *needs_resync.write().unwrap() = true;
                    }
                    // Update our version need be
                    // Follow docs conventions and store the last resourceVersion
                    // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
                    else if let Some(nv) = new_version {
                        *version.write().unwrap() = nv;
                    }

                    event
                }))
            }
            Err(e) => {
                warn!("Poll error: {:?}", e);
                // Set that we need a resync for the next poll
                // which will then reset our resource version and
                // wait a bit
                *self.needs_resync.write().unwrap() = false;
                Err(e)
            }
        }
    }

    /// Reset the resourceVersion to current and clear the event queue
    pub async fn reset(&self) -> Result<()> {
        // Fetch a new initial version:
        let initial = self.get_resource_version().await?;
        *self.version.write().unwrap() = initial;
        Ok(())
    }

    /// Return the current version
    pub fn version(&self) -> String {
        self.version.read().unwrap().clone()
    }

    /// Init helper
    async fn get_resource_version(&self) -> Result<String> {
        let req = self.resource.list_zero_resource_entries(&self.params)?;

        // parse to void a ResourceList into void except for Metadata
        let res = self.client.request::<ObjectList<Void>>(req).await?;

        let version = res.metadata.resourceVersion.unwrap_or_else(|| "0".into());
        debug!(
            "Got fresh resourceVersion={} for {}",
            version, self.resource.resource
        );
        Ok(version)
    }
}
