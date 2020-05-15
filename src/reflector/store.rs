use super::ObjectRef;
use dashmap::DashMap;
use derivative::Derivative;
use k8s_openapi::Resource;
use std::fmt::Debug;
use std::sync::Arc;

#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""), Clone)]
pub struct Store<K: Resource> {
    // DashMap isn't async-aware, but that's fine as long
    // as we never hold the lock over an async/await boundary
    pub(crate) store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: Clone + Resource> Store<K> {
    #[must_use]
    pub fn get(&self, key: &ObjectRef<K>) -> Option<K> {
        // Clone to let go of the entry lock ASAP
        self.store.get(key).map(|entry| entry.value().clone())
    }
}
