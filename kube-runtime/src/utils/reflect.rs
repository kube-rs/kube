use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use futures::{ready, Future, Stream, TryStream};
use pin_project::pin_project;

use crate::{
    reflector::{store::Writer, ObjectRef, Store},
    watcher::{Error, Event},
};
use async_broadcast::{InactiveReceiver, Receiver, Sender};
use kube_client::Resource;

/// Stream returned by the [`reflect`](super::WatchStreamExt::reflect) method
#[pin_project]
pub struct Reflect<St, K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    stream: St,
    writer: Writer<K>,
}

impl<St, K> Reflect<St, K>
where
    St: TryStream<Ok = Event<K>>,
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(stream: St, writer: Writer<K>) -> Reflect<St, K> {
        Self { stream, writer }
    }
}

impl<St, K> Stream for Reflect<St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        me.stream.as_mut().poll_next(cx).map_ok(move |event| {
            me.writer.apply_watcher_event(&event);
            event
        })
    }
}

/// Stream returned by the [`reflect`](super::WatchStreamExt::reflect) method
#[pin_project]
pub struct SharedReflect<St, K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    stream: St,
    writer: Writer<K>,
    tx: Sender<ObjectRef<K>>,
    rx: InactiveReceiver<ObjectRef<K>>,
}

impl<St, K> SharedReflect<St, K>
where
    St: TryStream<Ok = Event<K>>,
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(stream: St, writer: Writer<K>, buf_size: usize) -> SharedReflect<St, K> {
        let (tx, rx) = async_broadcast::broadcast(buf_size);
        Self {
            stream,
            writer,
            tx,
            rx: rx.deactivate(),
        }
    }

    pub fn subscribe(&self) -> SubscribeHandle<K> {
        // Note: broadcast::Sender::new_receiver() will return a new receiver
        // that _will not_ replay any messages in the channel, effectively
        // starting from the latest message.
        //
        // Since we create a reader and a writer when calling reflect_shared()
        // this should be fine. All subsequent clones should go through
        // SubscribeHandle::clone() to get a receiver that replays all of the
        // messages in the channel.
        SubscribeHandle::new(self.writer.as_reader(), self.tx.new_receiver())
    }
}

impl<St, K> Stream for SharedReflect<St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        let next = me.stream.as_mut().poll_next(cx).map_ok(move |event| {
            me.writer.apply_watcher_event(&event);
            event
        });
        let ev = match ready!(next) {
            Some(Ok(event)) => event,
            None => return Poll::Ready(None),
            Some(Err(error)) => return Poll::Ready(Some(Err(error))),
        };

        match &ev {
            Event::Applied(obj) | Event::Deleted(obj) => {
                // No error handling for now
                // Future resolves to a Result<Option<v>> if explicitly marked
                // as non-blocking
                let _ = ready!(me.tx.broadcast(ObjectRef::from_obj(obj)).as_mut().poll(cx));
            }
            Event::Restarted(obj_list) => {
                for obj in obj_list.iter().map(ObjectRef::from_obj) {
                    let _ = ready!(me.tx.broadcast(obj).as_mut().poll(cx));
                }
            }
        }

        Poll::Ready(Some(Ok(ev)))
    }
}

#[pin_project]
pub struct SubscribeHandle<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    rx: Receiver<ObjectRef<K>>,
    reader: Store<K>,
}

impl<K> Clone for SubscribeHandle<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    fn clone(&self) -> Self {
        SubscribeHandle::new(self.reader.clone(), self.rx.clone())
    }
}

impl<K> SubscribeHandle<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(reader: Store<K>, rx: Receiver<ObjectRef<K>>) -> SubscribeHandle<K> {
        Self { reader, rx }
    }

    pub fn reader(&self) -> Store<K> {
        self.reader.clone()
    }
}

impl<K> Stream for SubscribeHandle<K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    type Item = Arc<K>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        // If we use try_recv() here we could return Poll::Ready(Error) and let
        // the controller's trigger_backoff come into play (?)
        match ready!(me.rx.as_mut().poll_next(cx)) {
            Some(obj_ref) => Poll::Ready(me.reader.get(&obj_ref)),
            None => Poll::Ready(None),
        }
    }
}


#[cfg(test)]
pub(crate) mod test {
    use std::{task::Poll, vec};

    use super::{Error, Event, Reflect};
    use crate::reflector;
    use futures::{pin_mut, poll, stream, StreamExt};
    use k8s_openapi::api::core::v1::Pod;

    fn testpod(name: &str) -> Pod {
        let mut pod = Pod::default();
        pod.metadata.name = Some(name.to_string());
        pod
    }

    #[tokio::test]
    async fn reflect_passes_events_through() {
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![foo, bar])),
        ]);
        let (reader, writer) = reflector::store();

        let reflect = Reflect::new(st, writer);
        pin_mut!(reflect);
        assert_eq!(reader.len(), 0);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(reader.len(), 1);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        assert_eq!(reader.len(), 1);

        let restarted = poll!(reflect.next());
        assert!(matches!(restarted, Poll::Ready(Some(Ok(Event::Restarted(_))))));
        assert_eq!(reader.len(), 2);

        assert!(matches!(poll!(reflect.next()), Poll::Ready(None)));
        assert_eq!(reader.len(), 2);
    }
}
