use crate::{
    api::{
        resource::{ListParams, Resource},
        Meta,
    },
    client::APIClient,
    runtime::informer::Informer,
};
use serde::de::DeserializeOwned;

pub struct Controller<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    client: APIClient,
    resource: Resource,
    reconciler: Box<dyn Fn()>,
    watches: Vec<Informer<K>>,
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
            reconciler: Box::new(|| ()),
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

    // TODO: callback fn to reconcile
    // needs to just call it with "name" + namespace
    // TODO: let users pass in their own state?
    pub fn reconciler<CB: 'static + Fn()>(mut self, cb: CB) -> Self {
        self.reconciler = Box::new(cb);
        self
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
