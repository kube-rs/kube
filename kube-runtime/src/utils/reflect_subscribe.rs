use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc};

use futures::{ready, Future, Stream, TryStream};
use pin_project::pin_project;
use tokio::time;
use tracing::{debug, error, instrument, trace};

use crate::{
    reflector::{store::Writer, ObjectRef, Store},
    watcher::{Error, Event},
};
use async_broadcast::{InactiveReceiver, Receiver, Sender};
use kube_client::Resource;

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

    #[pin]
    sleep: time::Sleep,
    buffer: VecDeque<ObjectRef<K>>,
    deadline: time::Duration,
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
            deadline: time::Duration::from_secs(10),
            sleep: time::sleep(time::Duration::ZERO),
            buffer: VecDeque::new(),
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

    #[instrument(
        name = "shared_stream",
        skip_all, 
        fields(active_readers = %self.tx.receiver_count(),
        inner_queue_depth = %self.buffer.len())
    )]
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
                        trace!(deadline_ms = %this.deadline.as_millis(), "Root stream's buffer is full, retrying with a deadline");
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
                return Poll::Ready(None);
            },
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
        let mut this = self.project();
        match ready!(this.rx.as_mut().poll_next(cx)) {
            Some(obj_ref) => this.reader
                    .get(&obj_ref)
                    .map(|obj| Poll::Ready(Some(obj)))
                    .unwrap_or(Poll::Pending),
            None => Poll::Ready(None)
        }
    }
}


#[cfg(test)]
pub(crate) mod test {
    use std::{task::Poll, vec};

    use super::{Error, Event};
    use crate::{reflector, utils::SharedReflect};
    use futures::{pin_mut, poll, stream, StreamExt};
    use k8s_openapi::api::core::v1::Pod;

    const TEST_BUFFER_SIZE: usize = 10;

    fn testpod(name: &str) -> Pod {
        let mut pod = Pod::default();
        pod.metadata.name = Some(name.to_string());
        pod
    }

    /*
     * A list of tests:
     * Happy Path:
     * - events are passed through (including errors);
     *   - And on None it all works well
     * - objects are shared through N subscribers;
     * - objects are shared through N subscribers but deletes don't do anything;
     * - when main stream shuts down readers can still receive
     * Pathological cases
     * - events are passed through on overflow and readers recover after delay;
     * ( any chance to see how many times `sleep` has been called?)
     * - when main stream shuts down readers can still receive after
     * backpressure is applied (?) 
     *
     * Integration tests:
     * - Three controllers. Can we get an integration test set-up with owned streams? */

    #[tokio::test]
    async fn shared_reflect_passes_events_through() {

    }
    async fn reflect_passes_events_through() {
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![foo, bar])),
        ]);
        let (reader, writer) = reflector::store();

        let reflect = SharedReflect::new(st, writer, TEST_BUFFER_SIZE);
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
