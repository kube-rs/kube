use crate::{
    api::{
        resource::{ListParams, Resource},
        Meta,
    },
    client::APIClient,
    runtime::informer::Informer,
    Result,
};
use futures::{stream, Stream};
use serde::de::DeserializeOwned;

#[derive(Clone)]
pub struct Controller<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    client: APIClient,
    resource: Resource,
    watches: Vec<Informer<K>>,
}

pub struct ReconcileEvent {
    pub name: String,
    pub namespace: Option<String>,
}

impl<K> Controller<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    /// Create a controller with a kube client on a kube resource
    pub fn new(client: APIClient, r: Resource) -> Self {
        Controller {
            client: client,
            resource: r,
            watches: vec![],
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
        // TODO: poll informers in parallel, have them push to a joint queue
        // TODO: debounce and read from joint queue
        // TODO: pass on debounced events to stream
        Ok(stream::empty())
    }

    /// Initialize
    pub fn init(self) -> Self {
        info!("Starting Controller for {:?}", self.resource);
        // TODO: init main informer
        // TODO: init all watchers and link them up
        // TODO: queue up events
        // TODO: debounce events
        // TODO: trigger events
        self
    }
}
