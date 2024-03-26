use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc};

use futures::{ready, Future, Stream, StreamExt, TryStream};
use pin_project::pin_project;
use tokio::time;
use tracing::{debug, error, trace};

use crate::{
    reflector::{store::Writer, Lookup, ObjectRef, Store},
    watcher::{Error, Event},
};
use async_broadcast::{InactiveReceiver, Receiver, Sender};

/// Stream returned by the [`reflect`](super::WatchStreamExt::reflect) method
#[pin_project]
pub struct ReflectDispatcher<St, K>
where
    K: Lookup + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    #[pin]
    stream: St,
    writer: Writer<K>,
    tx: Sender<ObjectRef<K>>,
    rx: InactiveReceiver<ObjectRef<K>>,

    #[pin]
    sleep: time::Sleep,
    buffer: VecDeque<ObjectRef<K>>,
    deadline: time::Duration,
}

impl<St, K> ReflectDispatcher<St, K>
where
    St: Stream<Item = Result<Event<K>, Error>> + 'static,
    K: Lookup + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
{
    pub(super) fn new(stream: St, writer: Writer<K>, buf_size: usize) -> ReflectDispatcher<St, K> {
        let (tx, rx) = async_broadcast::broadcast(buf_size);
        Self {
            stream,
            writer,
            tx,
            rx: rx.deactivate(),
            deadline: time::Duration::from_secs(10),
            sleep: time::sleep(time::Duration::ZERO),
            buffer: VecDeque::new(),
        }
    }

    pub fn subscribe(&self) -> ReflectHandle<K> {
        // Note: broadcast::Sender::new_receiver() will return a new receiver
        // that _will not_ replay any messages in the channel, effectively
        // starting from the latest message.
        //
        // Since we create a reader and a writer when calling reflect_shared()
        // this should be fine. All subsequent clones should go through
        // ReflectHandle::clone() to get a receiver that replays all of the
        // messages in the channel.
        ReflectHandle::new(self.writer.as_reader(), self.tx.new_receiver())
    }
}

impl<St, K> Stream for ReflectDispatcher<St, K>
where
    K: Lookup + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            if let Some(msg) = this.buffer.pop_front() {
                match this.tx.try_broadcast(msg) {
                    Ok(_) => {
                        trace!("Broadcast value");
                    }
                    Err(async_broadcast::TrySendError::Full(msg)) => {
                        // When the broadcast buffer is full, retry with a
                        // deadline.
                        //
                        // First, push the msg back to the front of the buffer
                        // so ordering is preserved.
                        this.buffer.push_front(msg);
                        trace!(
                        deadline_ms = %this.deadline.as_millis(),
                        queue_depth = %this.buffer.len(),
                        active_readers = %this.tx.receiver_count(),
                        "Root stream's buffer is full, retrying with a deadline"
                        );
                        ready!(this.sleep.as_mut().poll(cx));
                        error!("Shared stream cannot make progress; ensure subscribers are being driven");
                        // Reset timer
                        this.sleep.as_mut().reset(time::Instant::now() + *this.deadline);
                    }
                    Err(error) if error.is_disconnected() => {
                        // When the broadcast channel is disconnected, we have
                        // no active receivers. We should clear the buffer and
                        // avoid writing to the channel.
                        this.buffer.clear();
                        debug!("No active readers subscribed to shared stream");
                    }
                    _ => {
                        // Other possible error is a closed channel.
                        // We should never hit this since we are holding a
                        // writer and an inactive reader.
                    }
                }
            } else {
                break;
            }
        }

        let next = this.stream.as_mut().poll_next(cx).map_ok(|event| {
            this.writer.apply_watcher_event(&event);
            event
        });

        let event = match ready!(next) {
            Some(Ok(event)) => event,
            None => {
                tracing::info!("Stream terminated");
                this.tx.close();
                return Poll::Ready(None);
            }
            Some(Err(error)) => return Poll::Ready(Some(Err(error))),
        };

        match &event {
            // Only deal with Deleted events
            Event::Applied(obj) => {
                let obj_ref = ObjectRef::from_obj(obj);
                match this.tx.try_broadcast(obj_ref) {
                    Ok(_) => {}
                    Err(async_broadcast::TrySendError::Full(msg)) => {
                        debug!(
                            "Attempted to write to subscribers with no buffer space; applying backpressure"
                        );
                        this.buffer.push_back(msg);
                    }
                    // Channel is closed or we have no active readers.
                    // In both cases there's not much we can do, so drive the
                    // watch strem.
                    _ => {}
                }
            }
            Event::Restarted(obj_list) => {
                let obj_list = obj_list.iter().map(ObjectRef::from_obj);
                this.buffer.extend(obj_list);
                loop {
                    if let Some(msg) = this.buffer.pop_front() {
                        match this.tx.try_broadcast(msg) {
                            Ok(_) => {}
                            Err(async_broadcast::TrySendError::Full(msg)) => {
                                debug!(
                            "Attempted to write to subscribers with no buffer space; applying backpressure"
                            );
                                this.buffer.push_front(msg);
                                break;
                            }
                            _ => {}
                        }
                    } else {
                        break;
                    }
                }
            }
            // Delete events should refresh the store. There is no need to propagate
            // them to subscribers since we have already updated the store by this
            // point.
            _ => {}
        };

        Poll::Ready(Some(Ok(event)))
    }
}

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
    use std::{sync::Arc, task::Poll, vec};

    use super::{Error, Event};
    use crate::{reflector, utils::ReflectDispatcher};
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

        let (reader, writer) = reflector::store();
        let reflect = ReflectDispatcher::new(st, writer, 10);
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

        let (_, writer) = reflector::store();
        let reflect = ReflectDispatcher::new(st, writer, 10);
        pin_mut!(reflect);
        let subscriber = reflect.subscribe();
        pin_mut!(subscriber);

        // Deleted events should be skipped by subscriber.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Deleted(_))))
        ));
        assert!(matches!(poll!(subscriber.next()), Poll::Pending));

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

        let (_, writer) = reflector::store();
        let reflect = ReflectDispatcher::new(st, writer, 10);

        // We pin the reflector on the heap to make it easier to drop it.
        let mut reflect = Box::pin(reflect);
        let subscriber = reflect.subscribe();
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

        let (_, writer) = reflector::store();
        let reflect = ReflectDispatcher::new(st, writer, 1);
        pin_mut!(reflect);
        let subscriber = reflect.subscribe();
        pin_mut!(subscriber);
        let subscriber_slow = reflect.subscribe();
        pin_mut!(subscriber_slow);

        assert_eq!(poll!(subscriber.next()), Poll::Pending);
        assert_eq!(poll!(subscriber_slow.next()), Poll::Pending);

        // Poll first subscriber, but not the second.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(poll!(subscriber.next()), Poll::Ready(Some(foo.clone())));
        // One subscriber is not reading, so we need to apply backpressure until
        // channel has capacity.
        //
        // At this point, the buffer is full. Polling again will trigger the
        // backpressure logic. This means, next event will be returned, but no
        // more progress will be made after that until subscriber_slow catches
        // up.
        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Restarted(_))))
        ));
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
