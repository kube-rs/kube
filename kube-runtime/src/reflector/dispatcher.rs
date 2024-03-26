use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use futures::{ready, Stream};
use pin_project::pin_project;
use tokio::time;

use crate::reflector::{ObjectRef, Store};
use async_broadcast::{Receiver, Sender};
use kube_client::Resource;

pub(crate) struct Dispatcher<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    tx: Sender<ObjectRef<K>>,
    deadline: time::Duration,
}

impl<K> Dispatcher<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    pub(crate) fn new(tx: Sender<ObjectRef<K>>, deadline: time::Duration) -> Dispatcher<K> {
        Self { tx, deadline }
    }

    pub(crate) async fn send(&mut self, value: ObjectRef<K>) {
        self.tx.broadcast_direct(value).await;
    }
}

#[pin_project]
pub struct ReflectHandle<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    rx: Receiver<ObjectRef<K>>,
    reader: Store<K>,
}

impl<K> Clone for ReflectHandle<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    fn clone(&self) -> Self {
        ReflectHandle::new(self.reader.clone(), self.rx.clone())
    }
}

impl<K> ReflectHandle<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(reader: Store<K>, rx: Receiver<ObjectRef<K>>) -> ReflectHandle<K> {
        Self { reader, rx }
    }

    pub fn reader(&self) -> Store<K> {
        self.reader.clone()
    }
}

impl<K> Stream for ReflectHandle<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    type Item = Arc<K>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match ready!(this.rx.as_mut().poll_next(cx)) {
            Some(obj_ref) => this
                .reader
                .get(&obj_ref)
                .map(|obj| Poll::Ready(Some(obj)))
                .unwrap_or(Poll::Pending),
            None => Poll::Ready(None),
        }
    }
}
