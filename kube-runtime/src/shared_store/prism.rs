use crate::shared_store::ready_token::ReadyState;
use crate::watcher;
use crate::watcher::Event;
use futures::stream::BoxStream;
use futures::{stream, Stream, StreamExt};
use std::fmt::Debug;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::broadcast;

/// Wraps a Reflector stream and enables it to split out so that additional subscribers can be added.
/// This allows Controllers to subscribe to the same stream of events as the Reflector, without
/// having to own the original stream.
///
/// We only forward Ok events along the subscriptions and let the primary Reflector stream handle any
/// errors.
// todo - not sure about struct name?
pub struct Prism<K> {
    reflector: BoxStream<'static, watcher::Result<Event<K>>>,
    ready_state: ReadyState,
    sender: broadcast::Sender<Event<K>>,
}

impl<K: Clone> Prism<K> {
    pub fn new(
        reflector: impl Stream<Item = watcher::Result<Event<K>>> + Send + 'static,
        ready_state: ReadyState,
    ) -> Self {
        let (sender, _) = broadcast::channel(100);

        Prism {
            reflector: reflector.boxed(),
            sender,
            ready_state,
        }
    }

    pub fn subscribe(&self) -> impl Stream<Item = Event<K>> {
        stream::unfold(self.sender.subscribe(), |mut rx| async move {
            match rx.recv().await {
                Ok(event) => Some((event, rx)),
                Err(_) => None,
            }
        })
    }
}

impl<K: Clone + Debug> Stream for Prism<K> {
    type Item = watcher::Result<Event<K>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = self.reflector.poll_next_unpin(cx);

        if let Poll::Ready(Some(Ok(event))) = &item {
            self.ready_state.ready();
            self.sender.send((*event).clone()).ok();
        }

        item
    }
}
