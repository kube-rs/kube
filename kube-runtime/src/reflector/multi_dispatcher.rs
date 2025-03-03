use std::{
    hash::Hash,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::watcher::Event;
use async_broadcast::{InactiveReceiver, Sender};
use futures::{lock::Mutex, Stream, StreamExt as _};
use kube_client::{api::DynamicObject, Resource};
use serde::de::DeserializeOwned;

use crate::watcher;

use super::{dispatcher::TypedReflectHandle, Store};

#[derive(Clone)]
pub struct MultiDispatcher {
    dispatch_tx: Sender<Event<DynamicObject>>,
    // An inactive reader that prevents the channel from closing until the
    // writer is dropped.
    _dispatch_rx: InactiveReceiver<Event<DynamicObject>>,
}

impl MultiDispatcher {
    #[must_use]
    pub fn new(buf_size: usize) -> Self {
        // Create a broadcast (tx, rx) pair
        let (mut dispatch_tx, dispatch_rx) = async_broadcast::broadcast(buf_size);
        // The tx half will not wait for any receivers to be active before
        // broadcasting events. If no receivers are active, events will be
        // buffered.
        dispatch_tx.set_await_active(false);
        Self {
            dispatch_tx,
            _dispatch_rx: dispatch_rx.deactivate(),
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
        let sub = TypedReflectHandle::new(self.dispatch_tx.new_receiver());
        let reader = sub.reader();
        (sub, reader)
    }

    /// Broadcast an event to any downstream listeners subscribed on the store
    pub(crate) async fn broadcast_event(&mut self, event: &watcher::Event<DynamicObject>) {
        match event {
            // Broadcast stores are pre-initialized
            watcher::Event::InitDone => {}
            ev => {
                let _ = self.dispatch_tx.broadcast_direct(ev.clone()).await;
            }
        }
    }
}

/// BroadcastStream allows to stream shared list of dynamic objects,
/// sources of which can be changed at any moment.
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
