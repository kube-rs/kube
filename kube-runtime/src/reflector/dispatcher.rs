use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

use futures::{ready, Stream};
use pin_project::pin_project;

use crate::reflector::{ObjectRef, Store};
use async_broadcast::Receiver;

use super::Lookup;

#[pin_project]
pub struct ReflectHandle<K>
where
    K: Lookup + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    rx: Receiver<ObjectRef<K>>,
    reader: Store<K>,
}

impl<K> Clone for ReflectHandle<K>
where
    K: Lookup + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    fn clone(&self) -> Self {
        ReflectHandle::new(self.reader.clone(), self.rx.clone())
    }
}

impl<K> ReflectHandle<K>
where
    K: Lookup + Clone,
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
    K: Lookup + Clone,
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

#[cfg(test)]
pub(crate) mod test {
    use crate::{
        watcher::{Error, Event},
        WatchStreamExt,
    };
    use std::{sync::Arc, task::Poll, vec};

    use crate::reflector;
    use futures::{pin_mut, poll, stream, StreamExt};
    use k8s_openapi::api::core::v1::Pod;

    fn testpod(name: &str) -> Pod {
        let mut pod = Pod::default();
        pod.metadata.name = Some(name.to_string());
        pod
    }

    #[tokio::test]
    async fn events_are_passed_through() {
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![foo, bar])),
        ]);

        let (reader, writer) = reflector::store_with_dispatch(10, Default::default());
        let reflect = st.reflect_dispatch(writer);
        pin_mut!(reflect);

        // Prior to any polls, we should have an empty store.
        assert_eq!(reader.len(), 0);
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));

        // Make progress and assert all events are seen
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

    #[tokio::test]
    async fn readers_yield_touched_objects() {
        // Readers should yield touched objects they receive from Stream events.
        //
        // NOTE: a Delete(_) event will be ignored if the item does not exist in
        // the cache. Same with a Restarted(vec![delete_item])
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Deleted(foo.clone())),
            Ok(Event::Applied(foo.clone())),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![foo.clone(), bar.clone()])),
        ]);

        let foo = Arc::new(foo);
        let bar = Arc::new(bar);

        let (_, writer) = reflector::store_with_dispatch(10, Default::default());
        let subscriber = writer.subscribe();
        let reflect = st.reflect_dispatch(writer);
        pin_mut!(reflect);
        pin_mut!(subscriber);

        // Deleted events should be skipped by subscriber.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Deleted(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Pending);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));

        // Errors are not propagated to subscribers.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        assert!(matches!(poll!(subscriber.next()), Poll::Pending));

        // Restart event will yield all objects in the list
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Restarted(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(bar.clone())));

        // When main channel is closed, it is propagated to subscribers
        assert!(matches!(poll!(reflect.next()), Poll::Ready(None)));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(None));
    }

    #[tokio::test]
    async fn readers_yield_when_tx_drops() {
        // Once the main stream is dropped, readers should continue to make
        // progress and read values that have been sent on the channel.
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Ok(Event::Restarted(vec![foo.clone(), bar.clone()])),
        ]);

        let foo = Arc::new(foo);
        let bar = Arc::new(bar);

        let (_, writer) = reflector::store_with_dispatch(10, Default::default());
        let subscriber = writer.subscribe();
        let mut reflect = Box::pin(st.reflect_dispatch(writer));
        pin_mut!(subscriber);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));

        // Restart event will yield all objects in the list. Broadcast values
        // without polling and then drop.
        //
        // First, subscribers should be pending.
        assert_eq!(poll!(subscriber.next()), Poll::Pending);
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Restarted(_))))
        ));
        drop(reflect);

        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(bar.clone())));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(None));
    }

    #[tokio::test]
    async fn reflect_applies_backpressure() {
        // When the channel is full, we should observe backpressure applied.
        //
        // This will be manifested by receiving Poll::Pending on the reflector
        // stream while the reader stream is not polled. Once we unblock the
        // buffer, the reflector will make progress.
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Ok(Event::Restarted(vec![foo.clone(), bar.clone()])),
        ]);

        let foo = Arc::new(foo);
        let bar = Arc::new(bar);

        let (_, writer) = reflector::store_with_dispatch(1, Default::default());
        let subscriber = writer.subscribe();
        let subscriber_slow = writer.subscribe();
        let reflect = st.reflect_dispatch(writer);
        pin_mut!(reflect);
        pin_mut!(subscriber);
        pin_mut!(subscriber_slow);

        assert_eq!(poll!(subscriber.next()), Poll::Pending);
        assert_eq!(poll!(subscriber_slow.next()), Poll::Pending);

        // Poll first subscriber, but not the second.
        //
        // The buffer can hold one value, so even if we have a slow subscriber,
        // we will still get an event from the root.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));
        // One subscriber is not reading, so we need to apply backpressure until
        // channel has capacity.
        //
        // At this point, the buffer is full. Polling again will trigger the
        // backpressure logic.
        assert!(matches!(poll!(reflect.next()), Poll::Pending));

        // Our "fast" subscriber will also have nothing else to poll until the
        // slower subscriber advances its pointer in the buffer.
        assert_eq!(poll!(subscriber.next()), Poll::Pending);

        // Advance slow reader
        assert_eq!(poll!(subscriber_slow.next()), Poll::Ready(Some(foo.clone())));

        // We now have room for only one more item. In total, the previous event
        // had two. We repeat the same pattern.
        assert!(matches!(poll!(reflect.next()), Poll::Pending));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));
        assert!(matches!(poll!(reflect.next()), Poll::Pending));
        assert_eq!(poll!(subscriber_slow.next()), Poll::Ready(Some(foo.clone())));
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Restarted(_))))
        ));
        // Poll again to drain the queue.
        assert!(matches!(poll!(reflect.next()), Poll::Ready(None)));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(bar.clone())));
        assert_eq!(poll!(subscriber_slow.next()), Poll::Ready(Some(bar.clone())));

        assert_eq!(poll!(subscriber.next()), Poll::Ready(None));
        assert_eq!(poll!(subscriber_slow.next()), Poll::Ready(None));
    }

    // TODO (matei): tests around cloning subscribers once a watch stream has already
    // been established. This will depend on the interfaces & impl so are left
    // out for now.
}
