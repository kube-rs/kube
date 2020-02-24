use crate::{
    api::{
        resource::{KubeObject, ObjectList, WatchEvent},
        Api, ListParams, NotUsed, RawApi,
    },
    client::APIClient,
    Result,
};

use futures::{lock::Mutex, TryStream, StreamExt};
use futures_timer::Delay;
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};

/// An event informer for a `Resource`
///
/// This watches a `Resource<K>`, by:
/// - seeding the intial resourceVersion with a list call (optional)
/// - keeping track of resourceVersions during every poll
/// - recovering when resourceVersions get desynced
#[derive(Clone)]
pub struct Informer<K>
where
    K: Clone + DeserializeOwned + KubeObject,
{
    version: Arc<Mutex<String>>,
    client: APIClient,
    resource: RawApi,
    params: ListParams,
    needs_resync: Arc<Mutex<bool>>,
    needs_retry: Arc<Mutex<bool>>,
    phantom: std::marker::PhantomData<K>,
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
            version: Arc::new(Mutex::new(0.to_string())),
            needs_resync: Arc::new(Mutex::new(false)),
            needs_retry: Arc::new(Mutex::new(false)),
            phantom: std::marker::PhantomData,
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
            version: Arc::new(Mutex::new(0.to_string())),
            needs_resync: Arc::new(Mutex::new(false)),
            needs_retry: Arc::new(Mutex::new(false)),
            phantom: std::marker::PhantomData,
        }
    }

    // builders for GetParams

    /// Configure the timeout for the list/watch call.
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// Defaults to 300s
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
        info!("Starting Informer for {:?}", self.resource);
        self.reset().await?;
        Ok(self)
    }

    /// Initialize from a prior version
    pub fn init_from(self, v: String) -> Self {
        info!("Recreating Informer for {:?} at {}", self.resource, v);

        // We need to block on this as our mutex needs go be async compatible
        futures::executor::block_on(async {
            *self.version.lock().await = v;
        });
        self
    }

    /// Start a single watch stream
    ///
    /// Opens a long polling GET and returns the complete WatchEvents as a Stream.
    /// You should always poll. When this call ends, call it again.
    /// Do not call it from more than one context.
    ///
    /// This function will handle error handling up to a point:
    /// - if we go out of history (410 Gone), we reset to latest
    /// - if we failed an initial poll, we will retry
    /// All real errors are bubbled up, as are WachEvent::Error instances.
    /// In the retry/reset cases we wait 10s between each attempt.
    ///
    /// If you need to track the `resourceVersion` you can use `Informer::version()`.
    pub async fn poll(&self) -> Result<impl TryStream<Item = Result<WatchEvent<K>>>> {
        trace!("Watching {:?}", self.resource);

        // First check if we need to backoff or reset our resourceVersion from last time
        {
            let mut needs_retry = self.needs_retry.lock().await;
            let mut needs_resync = self.needs_resync.lock().await;
            if *needs_resync || *needs_retry {
                // Try again in a bit
                let dur = Duration::from_secs(10);
                Delay::new(dur).await;
                // If we are outside history, start over from latest
                if *needs_resync {
                    self.reset().await?;
                }
                *needs_resync = false;
                *needs_retry = false;
            }
        }

        // Create our watch request
        let resource_version = self.version.lock().await.clone();
        let req = self.resource.watch(&self.params, &resource_version)?;

        // Clone Arcs for stream handling
        let version = self.version.clone();
        let needs_resync = self.needs_resync.clone();

        // Attempt to fetch our stream
        let stream = self.client.request_events::<WatchEvent<K>>(req).await;

        match stream {
            Ok(events) => {
                // Intercept stream elements to update internal resourceVersion
                Ok(events.then(move |event| {
                    // Need to clone our Arcs as they are consumed each loop
                    let needs_resync = needs_resync.clone();
                    let version = version.clone();
                    async move {
                        // Check if we need to update our version based on the incoming events
                        match &event {
                            Ok(WatchEvent::Added(o))
                            | Ok(WatchEvent::Modified(o))
                            | Ok(WatchEvent::Deleted(o)) => {
                                // Follow docs conventions and store the last resourceVersion
                                // https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes
                                if let Some(nv) = &o.meta().resourceVersion {
                                    *version.lock().await = nv.clone();
                                }
                            }
                            Ok(WatchEvent::Error(e)) => {
                                // 410 Gone => we need to restart from latest next call
                                if e.code == 410 {
                                    warn!("Stream desynced: {:?}", e);
                                    *needs_resync.lock().await = true;
                                }
                            }
                            Err(e) => {
                                warn!("Unexpected watch error: {:?}", e);
                            }
                        };
                        event
                    }
                }))
            }
            Err(e) => {
                warn!("Poll error: {:?}", e);
                // If we failed to do the main watch - try again later with same version
                *self.needs_retry.lock().await = false;
                Err(e)
            }
        }
    }

    /// Reset the resourceVersion to latest
    pub async fn reset(&self) -> Result<()> {
        let latest = self.get_resource_version().await?;
        *self.version.lock().await = latest;
        Ok(())
    }

    /// Return the current version
    pub fn version(&self) -> String {
        // We need to block on a future here quickly
        // to get a lock on our version
        futures::executor::block_on(async { self.version.lock().await.clone() })
    }

    /// Init helper
    async fn get_resource_version(&self) -> Result<String> {
        let req = self.resource.list_zero_resource_entries(&self.params)?;

        // parse to void a ResourceList into void except for Metadata
        let res = self.client.request::<ObjectList<NotUsed>>(req).await?;

        let version = res.metadata.resourceVersion.unwrap_or_else(|| "0".into());
        debug!(
            "Got fresh resourceVersion={} for {}",
            version, self.resource.resource
        );
        Ok(version)
    }
}
