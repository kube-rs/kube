use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::{collections::VecDeque, sync::Arc};

use futures::{ready, Future, Stream, TryStream};
use pin_project::pin_project;
use tokio::time;
use tracing::info;

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
    state: BroadcastState<K>,
    deadline: time::Duration,
}

#[pin_project(project = BroadcastStateProj)]
enum BroadcastState<K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    Reading,
    BlockedOnWrite {
        #[pin]
        sleep: time::Sleep,
        buffer: VecDeque<ObjectRef<K>>,
        event: Event<K>,
    },
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

impl<St, K> Stream for SharedReflect<St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone + Default,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        info!("Polling");
        loop {
            match this.state.as_mut().project() {
                // Continue reading
                BroadcastStateProj::Reading => {}
                BroadcastStateProj::BlockedOnWrite {
                    mut sleep,
                    buffer,
                    event,
                } => {
                    loop {
                        let c = buffer.len();
                        info!(count = %c, "Starting loop");
                        if buffer.is_empty() {
                            let event = event.to_owned();
                            info!("Switched to Reading");
                            this.state.set(BroadcastState::Reading);
                            return Poll::Ready(Some(Ok(event)));
                        }
                        let next = buffer.pop_front().unwrap();
                        match this.tx.try_broadcast(next) {
                            Ok(_) => {
                                let c = buffer.len();
                                info!(count = %c, "Sent it");
                            }
                            Err(async_broadcast::TrySendError::Full(msg)) => {
                                let c = buffer.len();
                                info!(count = %c, "oh nooo");
                                // Enqueue value back up
                                buffer.push_front(msg);
                                tracing::info!("Getting ready to be slept");
                                ready!(sleep.as_mut().poll(cx));
                                tracing::info!("Stream is stuck");
                                // Reset timer and re-start loop.
                                sleep.as_mut().reset(time::Instant::now() + *this.deadline);
                                return Poll::Pending;
                            }
                            _ => {}
                        }
                    }
                }
            }

            let next = this.stream.as_mut().poll_next(cx).map_ok(|event| {
                this.writer.apply_watcher_event(&event);
                event
            });

            let ev = match ready!(next) {
                Some(Ok(event)) => event,
                None => return Poll::Ready(None),
                Some(Err(error)) => return Poll::Ready(Some(Err(error))),
            };


            let buf = match &ev {
                Event::Applied(obj) | Event::Deleted(obj) => {
                    info!("Processing Applied | Deleted event");
                    let obj_ref = ObjectRef::from_obj(obj);
                    match this.tx.try_broadcast(obj_ref) {
                        Ok(_) => {
                            info!("First try in single event");
                            return Poll::Ready(Some(Ok(ev)));
                        }
                        Err(async_broadcast::TrySendError::Full(msg)) => {
                            info!("oh nooo, switch states");
                            let mut buf = VecDeque::new();
                            buf.push_back(msg);
                            buf
                        }
                        _ => return Poll::Pending,
                    }
                }
                Event::Restarted(obj_list) => {
                    info!("Processing restarted event");
                    let mut obj_list = obj_list
                        .iter()
                        .map(ObjectRef::from_obj)
                        .collect::<VecDeque<ObjectRef<K>>>();

                    loop {
                        if obj_list.is_empty() {
                            info!("First try very nice");
                            return Poll::Ready(Some(Ok(ev)));
                        }

                        let obj_ref = obj_list.pop_front().unwrap();
                        match this.tx.try_broadcast(obj_ref) {
                            Ok(_) => {}
                            Err(async_broadcast::TrySendError::Full(msg)) => {
                                obj_list.push_front(msg);
                                break obj_list;
                            }
                            _ => return Poll::Pending,
                        }
                    }
                }
            };

            info!("Switched to BlockedOnWrite");
            this.state.set(BroadcastState::BlockedOnWrite {
                sleep: tokio::time::sleep(*this.deadline),
                buffer: buf,
                event: ev,
            });
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
