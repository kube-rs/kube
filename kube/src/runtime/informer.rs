use crate::{
    api::{Api, ListParams, Meta, WatchEvent},
    Result,
};

use futures::{lock::Mutex, Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};

/// An event informer for a Kubernetes ['Api'] resource
///
/// This observes events on an `Api<K>` and tracks last seen versions.
/// As per the kubernetes documentation, this is an abstraction that can
/// [efficiently detect changes](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
///
/// In the case where kubernetes returns 410 Gone (desynced / watched for too long)
/// this object will reset the informer ensuring that it always keeps running.
///
/// This means that you might occasionally get some duplicate added events,
/// but we have configured timeouts such that this should not happen frequently.
///
/// On boot, the initial watch causes added events for every currently live object.
///
/// Because of https://github.com/clux/kube-rs/issues/219 we recommend you use this
/// with kubernetes >= 1.16 and watch bookmarks enabled.
#[derive(Clone)]
#[deprecated(note = "Replaced by kube_runtime::watcher", since = "0.38.0")]
pub struct Informer<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    version: Arc<Mutex<String>>,
    api: Api<K>,
    params: ListParams,
    needs_resync: Arc<Mutex<bool>>,
}

impl<K> Informer<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    /// Create an informer on an api resource
    pub fn new(api: Api<K>) -> Self {
        Informer {
            api,
            params: ListParams::default(),
            version: Arc::new(Mutex::new(0.to_string())),
            needs_resync: Arc::new(Mutex::new(false)),
        }
    }

    /// Modify the default watch parameters for the underlying watch
    pub fn params(mut self, lp: ListParams) -> Self {
        self.params = lp;
        self
    }

    /// Override the version to an externally tracked version
    ///
    /// Prefer not using this. Even if you track previous resource versions,
    /// you will miss deleted events if you have any downtime.
    ///
    /// Controllers/finalizers/ownerReferences are the preferred ways
    /// to garbage collect related resources.
    pub fn set_version(self, v: String) -> Self {
        debug!("Setting Informer version for {} to {}", self.api.resource.kind, v);

        // We need to block on this as our mutex needs go be async compatible
        futures::executor::block_on(async {
            *self.version.lock().await = v;
        });
        self
    }

    /// Reset the resourceVersion to 0
    ///
    /// This will trigger new Added events for all existing resources
    pub async fn reset(&self) {
        *self.version.lock().await = 0.to_string();
    }

    /// Return the current version
    pub fn version(&self) -> String {
        // We need to block on a future here quickly
        // to get a lock on our version
        futures::executor::block_on(async { self.version.lock().await.clone() })
    }

    /// Start a single watch stream
    ///
    /// Opens a long polling GET and returns a stream of WatchEvents.
    /// You should always poll. When this call ends, call it again.
    /// Do not call it from more than one context.
    ///
    /// All real errors are bubbled up, as are WachEvent::Error instances.
    /// If we are desynced we force a 10s wait 10s before starting the poll.
    ///
    /// If you need to track the `resourceVersion` you can use `Informer::version()`.
    pub async fn poll(&self) -> Result<impl Stream<Item = Result<WatchEvent<K>>>> {
        trace!("Watching {}", self.api.resource.kind);

        // First check if we need to backoff or reset our resourceVersion from last time
        {
            let mut needs_resync = self.needs_resync.lock().await;
            if *needs_resync {
                // Try again in a bit
                let dur = Duration::from_secs(10);
                tokio::time::delay_for(dur).await;
                // If we are outside history, start over from latest
                if *needs_resync {
                    self.reset().await;
                }
                *needs_resync = false;
            }
        }

        // Clone Arcs for stream handling
        let version = self.version.clone();
        let needs_resync = self.needs_resync.clone();

        // Start watching from our previous watch point
        let resource_version = self.version.lock().await.clone();
        let stream = self.api.watch(&self.params, &resource_version).await?;

        // Intercept stream elements to update internal resourceVersion
        let newstream = stream.then(move |event| {
            // Clone our Arcs for each event
            let needs_resync = needs_resync.clone();
            let version = version.clone();
            async move {
                // Check if we need to update our version based on the incoming events
                match &event {
                    Ok(WatchEvent::Added(o)) | Ok(WatchEvent::Modified(o)) | Ok(WatchEvent::Deleted(o)) => {
                        // always store the last seen resourceVersion
                        if let Some(nv) = Meta::resource_ver(o) {
                            *version.lock().await = nv.to_string();
                        }
                    }
                    Ok(WatchEvent::Bookmark(bm)) => {
                        // always store the last seen resourceVersion
                        *version.lock().await = bm.metadata.resource_version.clone();
                    }
                    Ok(WatchEvent::Error(e)) => {
                        // 410 Gone => we need to restart from latest next call
                        if e.code == 410 {
                            warn!("Stream desynced: {:?}", e);
                            *needs_resync.lock().await = true;
                        }
                    }
                    Err(e) => {
                        // All we seem to get here are:
                        // - EOFs (mostly solved with timeout enforcement + resyncs)
                        // - serde errors (bad struct use, on app side)
                        // Not much we can do about these here.
                        warn!("Unexpected watch error: {:?}", e);
                    }
                };
                event
            }
        });
        Ok(newstream)
    }
}
