use std::hash::Hash;

use kube_client::{api::DynamicObject, Resource};
use serde::de::DeserializeOwned;

use crate::watcher;

use super::{
    dispatcher::{DynamicDispatcher, TypedReflectHandle},
    Store,
};

#[derive(Clone)]
pub struct MultiDispatcher {
    dispatcher: DynamicDispatcher,
}

impl MultiDispatcher {
    #[must_use]
    pub fn new(buf_size: usize) -> Self {
        Self {
            dispatcher: DynamicDispatcher::new(buf_size),
        }
    }

    /// Return a handle to a typed subscriber
    ///
    /// Multiple subscribe handles may be obtained, by either calling
    /// `subscribe` multiple times, or by calling `clone()`
    ///
    /// This function returns a `Some` when the [`Writer`] is constructed through
    /// [`Writer::new_shared`] or [`store_shared`], and a `None` otherwise.
    #[must_use]
    pub fn subscribe<K>(&self) -> (TypedReflectHandle<K>, Store<K>)
    where
        K: Resource + Clone + DeserializeOwned,
        K::DynamicType: Eq + Clone + Hash + Default,
    {
        let sub = self.dispatcher.subscribe();
        let reader = sub.reader();
        (sub, reader)
    }

    /// Broadcast an event to any downstream listeners subscribed on the store
    pub(crate) async fn broadcast_event(&mut self, event: &watcher::Event<DynamicObject>) {
        self.dispatcher.broadcast(event.clone()).await
    }
}
