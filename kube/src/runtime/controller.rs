use futures::lock::Mutex;
use std::sync::Arc;
use std::collections::VecDeque;
use crate::{
    api::{
        resource::{ListParams, Resource},
        Meta,
        WatchEvent
    },
    client::APIClient,
    runtime::informer::Informer,
    Result,
};
use futures::{stream, Stream, StreamExt};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct Controller<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    client: APIClient,
    resource: Resource,
    watches: Vec<Informer<K>>,
    queue: Arc<Mutex<VecDeque<ReconcileEvent>>>,
}

#[derive(Debug)]
pub struct ReconcileEvent {
    pub name: String,
    pub namespace: Option<String>,
}

impl<K> Controller<K>
where
    K: Clone + DeserializeOwned + Meta + Send + Sync,
{
    /// Create a controller with a kube client on a kube resource
    pub fn new(client: APIClient, r: Resource) -> Self {
        Controller {
            client: client,
            resource: r,
            watches: vec![],
            queue: Default::default(),
        }
    }

    /// Create internal informers for an associated kube resource
    ///
    /// TODO: this needs to only find resources with a property matching root resource
    pub fn owns(mut self, r: Resource, lp: ListParams) -> Self {
        self.watches.push(Informer::new(self.client.clone(), lp, r));
        self
    }

    /// Poll reconcile events through all internal informers
    pub async fn poll(&self) -> Result<impl Stream<Item = Result<ReconcileEvent>>> {
        // 1. poll informers in parallel and push results to queue
        for inf in self.watches {
            // TODO: init all watchers and link them up;
            let queue = self.queue.clone();
            tokio::spawn(async move {
                let mut poll_i = inf.poll().await.unwrap().boxed();
                while let Some(ev) = poll_i.next().await {
                    // TODO: add_permit to a tokio semaphore here..
                    match ev {
                        Ok(WatchEvent::Added(o)) => {
                            (*queue.lock().await).push_back(ReconcileEvent {
                                name: Meta::name(&o),
                                namespace: Meta::namespace(&o)
                            });
                        }
                        _ => unimplemented!()
                    }
                }
            });
        }
        // TODO: debounce rx events
        // read from queue here with semaphore.acquire
        Ok(rx)
    }

    /// Initialize
    pub fn init(self) -> Self {
        info!("Starting Controller for {:?}", self.resource);
        // TODO: init main informer
        // TODO: queue up events
        // TODO: debounce events
        // TODO: trigger events
        self
    }
}
