use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use futures::{poll, ready, stream, Stream, TryStream};
use pin_project::pin_project;
use tokio::sync::broadcast;

use crate::{
    reflector::{store::Writer, ObjectRef, Store},
    watcher::{self, Error, Event},
};
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

    pub fn subscribe(&self) -> impl Stream<Item = ObjectRef<K>> {
        self.writer.subscribe()
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
            match &event {
                Event::Applied(obj) | Event::Deleted(obj) => {
                    me.writer.broadcast_tx.send(Some(ObjectRef::from_obj(obj))).ok();
                }
                Event::Restarted(obj_list) => {
                    for obj in obj_list.iter().map(ObjectRef::from_obj) {
                        me.writer.broadcast_tx.send(Some(obj)).ok();
                    }
                }
            };
            event
        })
    }
}

#[pin_project]
pub struct ReflectHandle<St, K>
where
    St: Stream,
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    stream: St,
    reader: Store<K>,
}

impl<St, K> ReflectHandle<St, K>
where
    St: Stream<Item = ObjectRef<K>>,
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(stream: St, reader: Store<K>) -> ReflectHandle<St, K> {
        Self { stream, reader }
    }

    pub fn reader(&self) -> Store<K> {
        self.reader.clone()
    }
}

impl<St, K> Stream for ReflectHandle<St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = ObjectRef<K>>,
{
    type Item = Arc<K>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        match ready!(me.stream.as_mut().poll_next(cx)) {
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
