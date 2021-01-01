use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project::pin_project;
use snafu::{Backtrace, ResultExt, Snafu};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};
use tokio::time::{self, Instant};
use tokio_util::time::delay_queue::{self, DelayQueue};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("timer failure: {}", source))]
    TimerError {
        source: time::error::Error,
        backtrace: Backtrace,
    },
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A request to re-emit `message` at a given `Instant` (`run_at`).
#[derive(Debug)]
pub struct ScheduleRequest<T> {
    pub message: T,
    pub run_at: Instant,
}

/// Internal metadata for a scheduled message.
struct ScheduledEntry {
    run_at: Instant,
    queue_key: delay_queue::Key,
}

#[pin_project(project = SchedulerProj)]
pub struct Scheduler<T, R> {
    /// Queue of already-scheduled messages.
    ///
    /// To ensure that the metadata is kept up-to-date, use `schedule_message` and
    /// `poll_pop_queue_message` rather than manipulating this directly.
    queue: DelayQueue<T>,
    /// Metadata for all currently scheduled messages. Used to detect duplicate messages.
    scheduled: HashMap<T, ScheduledEntry>,
    /// Messages that are scheduled to have happened, but have been held using `hold_unless`.
    pending: HashSet<T>,
    /// Incoming queue of scheduling requests.
    #[pin]
    requests: Fuse<R>,
}

impl<T, R: Stream> Scheduler<T, R> {
    fn new(requests: R) -> Self {
        Self {
            queue: DelayQueue::new(),
            scheduled: HashMap::new(),
            pending: HashSet::new(),
            requests: requests.fuse(),
        }
    }
}

impl<'a, T: Hash + Eq + Clone, R> SchedulerProj<'a, T, R> {
    /// Attempt to schedule a message into the queue.
    ///
    /// If the message is already in the queue then the earlier `request.run_at` takes precedence.
    fn schedule_message(&mut self, request: ScheduleRequest<T>) {
        if self.pending.contains(&request.message) {
            // Message is already pending, so we can't even expedite it
            return;
        }
        match self.scheduled.entry(request.message) {
            Entry::Occupied(mut old_entry) if old_entry.get().run_at >= request.run_at => {
                // Old entry will run after the new request, so replace it..
                let entry = old_entry.get_mut();
                // TODO: this should add a little delay here to actually debounce
                self.queue.reset_at(&entry.queue_key, request.run_at);
                entry.run_at = request.run_at;
            }
            Entry::Occupied(_old_entry) => {
                // Old entry will run before the new request, so ignore the new request..
            }
            Entry::Vacant(entry) => {
                // No old entry, we're free to go!
                let message = entry.key().clone();
                entry.insert(ScheduledEntry {
                    run_at: request.run_at,
                    queue_key: self.queue.insert_at(message, request.run_at),
                });
            }
        }
    }

    /// Attempt to retrieve a message from the queue.
    fn poll_pop_queue_message(
        &mut self,
        cx: &mut Context<'_>,
        can_take_message: impl Fn(&T) -> bool,
    ) -> Poll<Option<Result<T, time::error::Error>>> {
        if let Some(msg) = self.pending.iter().find(|msg| can_take_message(*msg)).cloned() {
            return Poll::Ready(Some(Ok(self.pending.take(&msg).unwrap())));
        }

        loop {
            match self.queue.poll_expired(cx) {
                Poll::Ready(Some(Ok(msg))) => {
                    let msg = msg.into_inner();
                    self.scheduled.remove(&msg).expect(
                    "Expired message was popped from the Scheduler queue, but was not in the metadata map",
                );
                    if can_take_message(&msg) {
                        break Poll::Ready(Some(Ok(msg)));
                    } else {
                        self.pending.insert(msg);
                    }
                }
                Poll::Ready(Some(Err(err))) => break Poll::Ready(Some(Err(err))),
                Poll::Ready(None) => {
                    break if self.pending.is_empty() {
                        Poll::Ready(None)
                    } else {
                        // There are still remaining pending messages, so we're not done quite yet..
                        Poll::Pending
                    };
                }
                Poll::Pending => break Poll::Pending,
            }
        }
    }
}

/// See [`Scheduler::hold_unless`]
pub struct HoldUnless<'a, T, R, C> {
    scheduler: Pin<&'a mut Scheduler<T, R>>,
    can_take_message: C,
}

impl<'a, T, R, C> Stream for HoldUnless<'a, T, R, C>
where
    T: Eq + Hash + Clone,
    R: Stream<Item = ScheduleRequest<T>>,
    C: Fn(&T) -> bool + Unpin,
{
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let can_take_message = &this.can_take_message;
        let mut scheduler = this.scheduler.as_mut().project();

        while let Poll::Ready(Some(request)) = scheduler.requests.as_mut().poll_next(cx) {
            scheduler.schedule_message(request);
        }

        match scheduler.poll_pop_queue_message(cx, &can_take_message) {
            Poll::Ready(Some(expired)) => Poll::Ready(Some(expired.context(TimerError))),
            Poll::Ready(None) => {
                if scheduler.requests.is_terminated() {
                    // The source queue has terminated, and all outstanding requests are done, so terminate
                    Poll::Ready(None)
                } else {
                    // The delay queue is empty, but we may get more requests in the future...
                    Poll::Pending
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, R> Scheduler<T, R>
where
    T: Eq + Hash + Clone,
    R: Stream<Item = ScheduleRequest<T>>,
{
    /// A filtered view of the [`Scheduler`], which will keep items "pending" if
    /// `can_take_message` returns `false`, allowing them to be handled as soon as
    /// they are ready.
    ///
    /// The returned [`HoldUnless`] is designed to be short-lived: it has no allocations, and
    /// no messages will be lost, even if it is reconstructed on each call to [`poll_next`](Self::poll_next).
    /// In fact, this is often desirable, to avoid long-lived borrows in `can_take_message`'s closure.
    ///
    /// NOTE: `can_take_message` should be considered fairly performance-sensitive, since
    /// it will generally be executed for each pending message, for each [`poll_next`](Self::poll_next).
    pub fn hold_unless<C: Fn(&T) -> bool>(self: Pin<&mut Self>, can_take_message: C) -> HoldUnless<T, R, C> {
        HoldUnless {
            scheduler: self,
            can_take_message,
        }
    }

    /// Checks whether `msg` is currently a pending message (held by `hold_unless`)
    #[cfg(test)]
    pub fn contains_pending(&self, msg: &T) -> bool {
        self.pending.contains(msg)
    }
}

impl<T, R> Stream for Scheduler<T, R>
where
    T: Eq + Hash + Clone,
    R: Stream<Item = ScheduleRequest<T>>,
{
    type Item = Result<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.hold_unless(|_| true)).poll_next(cx)
    }
}

/// Stream transformer that takes a message and `Instant` (in the form of a `ScheduleRequest`), and emits
/// the message at the specified `Instant`.
///
/// Objects are de-duplicated: if a message is submitted twice before being emitted then it will only be
/// emitted at the earlier of the two `Instant`s.
pub fn scheduler<T: Eq + Hash + Clone, S: Stream<Item = ScheduleRequest<T>>>(requests: S) -> Scheduler<T, S> {
    Scheduler::new(requests)
}

#[cfg(test)]
mod tests {
    use super::{scheduler, ScheduleRequest};
    use futures::{channel::mpsc, poll, stream, FutureExt, SinkExt, StreamExt};
    use std::task::Poll;
    use tokio::time::{advance, pause, Duration, Instant};

    fn unwrap_poll<T>(poll: Poll<T>) -> T {
        if let Poll::Ready(x) = poll {
            x
        } else {
            panic!("Tried to unwrap a pending poll!")
        }
    }

    #[tokio::test]
    async fn scheduler_should_hold_and_release_items() {
        pause();
        let mut scheduler = Box::pin(scheduler(stream::iter(vec![ScheduleRequest {
            message: 1_u8,
            run_at: Instant::now(),
        }])));
        assert!(!scheduler.contains_pending(&1));
        assert!(poll!(scheduler.as_mut().hold_unless(|_| false).next()).is_pending());
        assert!(scheduler.contains_pending(&1));
        assert_eq!(
            unwrap_poll(poll!(scheduler.as_mut().hold_unless(|_| true).next()))
                .unwrap()
                .unwrap(),
            1_u8
        );
        assert!(!scheduler.contains_pending(&1));
        assert!(scheduler.as_mut().hold_unless(|_| true).next().await.is_none());
    }

    #[tokio::test]
    async fn scheduler_should_not_reschedule_pending_items() {
        pause();
        let (mut tx, rx) = mpsc::unbounded::<ScheduleRequest<u8>>();
        let mut scheduler = Box::pin(scheduler(rx));
        tx.send(ScheduleRequest {
            message: 1,
            run_at: Instant::now(),
        })
        .await
        .unwrap();
        assert!(poll!(scheduler.as_mut().hold_unless(|_| false).next()).is_pending());
        tx.send(ScheduleRequest {
            message: 1,
            run_at: Instant::now(),
        })
        .await
        .unwrap();
        drop(tx);
        assert_eq!(scheduler.next().await.unwrap().unwrap(), 1);
        assert!(scheduler.next().await.is_none());
    }

    #[tokio::test]
    async fn scheduler_pending_message_should_not_block_head_of_line() {
        let mut scheduler = Box::pin(scheduler(stream::iter(vec![
            ScheduleRequest {
                message: 1,
                run_at: Instant::now(),
            },
            ScheduleRequest {
                message: 2,
                run_at: Instant::now(),
            },
        ])));
        assert_eq!(
            scheduler
                .as_mut()
                .hold_unless(|x| *x != 1)
                .next()
                .await
                .unwrap()
                .unwrap(),
            2
        );
    }

    #[tokio::test]
    async fn scheduler_should_emit_items_as_requested() {
        pause();
        let mut scheduler = scheduler(stream::iter(vec![
            ScheduleRequest {
                message: 1_u8,
                run_at: Instant::now() + Duration::from_secs(1),
            },
            ScheduleRequest {
                message: 2,
                run_at: Instant::now() + Duration::from_secs(3),
            },
        ]));
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), 1);
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), 2);
        // Stream has terminated
        assert!(scheduler.next().await.is_none());
    }

    #[tokio::test]
    async fn scheduler_dedupe_should_keep_earlier_item() {
        pause();
        let mut scheduler = scheduler(stream::iter(vec![
            ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(1),
            },
            ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(3),
            },
        ]));
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        scheduler.next().now_or_never().unwrap().unwrap().unwrap();
        // Stream has terminated
        assert!(scheduler.next().await.is_none());
    }

    #[tokio::test]
    async fn scheduler_dedupe_should_replace_later_item() {
        pause();
        let mut scheduler = scheduler(stream::iter(vec![
            ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(3),
            },
            ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(1),
            },
        ]));
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        scheduler.next().now_or_never().unwrap().unwrap().unwrap();
        // Stream has terminated
        assert!(scheduler.next().await.is_none());
    }

    #[tokio::test]
    async fn scheduler_dedupe_should_allow_rescheduling_emitted_item() {
        pause();
        let (mut schedule_tx, schedule_rx) = mpsc::unbounded();
        let mut scheduler = scheduler(schedule_rx);
        schedule_tx
            .send(ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(1),
            })
            .await
            .unwrap();
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        scheduler.next().now_or_never().unwrap().unwrap().unwrap();
        assert!(poll!(scheduler.next()).is_pending());
        schedule_tx
            .send(ScheduleRequest {
                message: (),
                run_at: Instant::now() + Duration::from_secs(1),
            })
            .await
            .unwrap();
        assert!(poll!(scheduler.next()).is_pending());
        advance(Duration::from_secs(2)).await;
        scheduler.next().now_or_never().unwrap().unwrap().unwrap();
        assert!(poll!(scheduler.next()).is_pending());
    }
}
