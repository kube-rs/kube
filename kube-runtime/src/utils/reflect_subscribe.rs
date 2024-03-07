use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc};

use futures::{ready, Future, Stream, TryStream};
use pin_project::pin_project;
use tokio::time;

use crate::{
    reflector::{store::Writer, ObjectRef, Store},
    watcher::{Error, Event},
};
use async_broadcast::{InactiveReceiver, Receiver, Sender};
use kube_client::Resource;


/// Stream returned by the [`reflect`](super::WatchStreamExt::reflect) method
#[pin_project]
pub struct SharedReflect<'a, St, K>
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
    state: BroadcastState<'a, K>,
    deadline: time::Duration,
}

#[pin_project(project = BroadcastStateProj)]
enum BroadcastState<'a, K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    Reading,
    Writing {
        #[pin]
        sleep: time::Sleep,
        #[pin]
        send_fut: async_broadcast::Send<'a, ObjectRef<K>>,
        event: Event<K>,
    },
    WritingBuffered {
        #[pin]
        sleep: time::Sleep,
        #[pin]
        send_fut: async_broadcast::Send<'a, ObjectRef<K>>,
        items: VecDeque<ObjectRef<K>>,
        event: Event<K>,
    },
}

impl<'a, St, K> SharedReflect<'a, St, K>
where
    St: TryStream<Ok = Event<K>>,
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(stream: St, writer: Writer<K>, buf_size: usize) -> SharedReflect<'a, St, K> {
        let (tx, rx) = async_broadcast::broadcast(buf_size);
        Self {
            stream,
            writer,
            tx,
            rx: rx.deactivate(),
            state: BroadcastState::Reading,
            deadline: time::Duration::from_secs(2),
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

impl<St, K> Stream for SharedReflect<'_, St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        loop {
            match this.state.as_mut().project() {
                // Continue reading
                BroadcastStateProj::Reading => {}
                BroadcastStateProj::Writing {
                    mut sleep,
                    mut send_fut,
                    event,
                } => match send_fut.as_mut().poll(cx) {
                    Poll::Ready(_) => {
                        tracing::trace!("Broadcast value");
                        this.state.set(BroadcastState::Reading);
                        return Poll::Ready(Some(Ok(event.clone())));
                    }
                    Poll::Pending => {
                        ready!(sleep.as_mut().poll(cx));
                        tracing::debug!("Stream is stuck, clear your buffer");
                        sleep.as_mut().reset(time::Instant::now() + *this.deadline);
                    }
                },
                BroadcastStateProj::WritingBuffered {
                    mut sleep,
                    mut send_fut,
                    items,
                    event,
                } => match send_fut.as_mut().poll(cx) {
                    Poll::Ready(_) => {
                        let next = items.pop_front().map(|obj| this.tx.broadcast_direct(obj));
                        let left = items.len();
                        tracing::trace!(items_left = %left, "Broadcast buffered value");
                        match next {
                            Some(next) => this.state.set(BroadcastState::WritingBuffered {
                                sleep: time::sleep(*this.deadline),
                                send_fut: next,
                                items: *items,
                                event: *event,
                            }),
                            Some(next) if items.is_empty() => this.state.set(BroadcastState::Writing {
                                sleep: time::sleep(*this.deadline),
                                send_fut: next,
                                event: *event,
                            }),

                            None => {}
                        }
                        return Poll::Pending;
                    }
                    Poll::Pending => {
                        ready!(sleep.as_mut().poll(cx));
                        tracing::debug!("Stream is stuck, clear your buffer");
                        sleep.as_mut().reset(time::Instant::now() + *this.deadline);
                    }
                },
            }


            let next = this.stream.as_mut().poll_next(cx).map_ok(move |event| {
                this.writer.apply_watcher_event(&event);
                event
            });

            let ev = match ready!(next) {
                Some(Ok(event)) => event,
                None => return Poll::Ready(None),
                Some(Err(error)) => return Poll::Ready(Some(Err(error))),
            };


            match &ev {
                Event::Applied(obj) | Event::Deleted(obj) => this.state.set(BroadcastState::Writing {
                    sleep: time::sleep(*this.deadline),
                    send_fut: this.tx.broadcast_direct(ObjectRef::from_obj(obj)),
                    event: ev,
                }),
                Event::Restarted(obj_list) => {
                    let mut obj_list = obj_list
                        .iter()
                        .map(ObjectRef::from_obj)
                        .collect::<VecDeque<ObjectRef<K>>>();
                    let next = obj_list.pop_front().map(|obj| this.tx.broadcast_direct(obj));
                    if let Some(next) = next {
                        this.state.set(BroadcastState::WritingBuffered {
                            sleep: time::sleep(*this.deadline),
                            send_fut: next,
                            items: obj_list,
                            event: ev,
                        })
                    }
                }
            }
        }
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
