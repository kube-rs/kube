use crate::watcher;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::stream::BoxStream;
use futures::{stream, Stream, StreamExt};
use std::fmt::Debug;
use tokio::sync::broadcast;

pub struct StreamSubscribable<K> {
    reflector: BoxStream<'static, watcher::Result<watcher::Event<K>>>,
    sender: broadcast::Sender<watcher::Event<K>>,
}

impl<K: Clone> StreamSubscribable<K> {
    pub fn new(reflector: impl Stream<Item = watcher::Result<watcher::Event<K>>> + Send + 'static) -> Self {
        let (sender, _) = broadcast::channel(100);

        StreamSubscribable {
            reflector: reflector.boxed(),
            sender,
        }
    }

    pub fn subscribe_ok(&self) -> impl Stream<Item = watcher::Event<K>> {
        stream::unfold(self.sender.subscribe(), |mut rx| async move {
            match rx.recv().await {
                Ok(event) => Some((event, rx)),
                Err(_) => None,
            }
        })
    }
}

impl<K: Clone + Debug> Stream for StreamSubscribable<K> {
    type Item = watcher::Result<watcher::Event<K>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = self.reflector.poll_next_unpin(cx);

        if let Poll::Ready(Some(Ok(event))) = &item {
            self.sender.send((*event).clone()).ok();
        }

        item
    }
}
