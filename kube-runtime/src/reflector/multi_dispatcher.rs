use std::{
    hash::Hash,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use futures::{lock::Mutex, FutureExt, Stream, StreamExt as _};
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
        match event {
            // Broadcast stores are pre-initialized
            watcher::Event::InitDone => {}
            ev => self.dispatcher.broadcast(ev.clone()).await,
        }
    }
}

/// See [`Scheduler::hold`]
pub struct BroadcastStream<W> {
    pub stream: Arc<Mutex<W>>,
}

impl<W> Clone for BroadcastStream<W> {
    fn clone(&self) -> Self {
        Self {
            stream: self.stream.clone(),
        }
    }
}

impl<W> BroadcastStream<W>
where
    W: Stream<Item = watcher::Result<watcher::Event<DynamicObject>>> + Unpin,
{
    pub fn new(stream: Arc<Mutex<W>>) -> Self {
        Self { stream }
    }
}

impl<W> Stream for BroadcastStream<W>
where
    W: Stream<Item = watcher::Result<watcher::Event<DynamicObject>>> + Unpin,
{
    type Item = W::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(mut stream) = self.stream.try_lock() {
            return stream.poll_next_unpin(cx);
        }

        Poll::Pending
    }
}
