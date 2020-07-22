use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project::pin_project;
use snafu::{Backtrace, ResultExt, Snafu};
use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
};
use time::delay_queue::Expired;
use tokio::time::{
    self,
    delay_queue::{self, DelayQueue},
    Instant,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("timer failure: {}", source))]
    TimerError {
        source: time::Error,
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
struct Scheduler<T, R> {
    /// Queue of already-scheduled messages.
    ///
    /// To ensure that the metadata is kept up-to-date, use `schedule_message` and
    /// `poll_pop_queue_message` rather than manipulating this directly.
    queue: DelayQueue<T>,
    /// Metadata for all currently scheduled messages. Used to detect duplicate messages.
    scheduled: HashMap<T, ScheduledEntry>,
    /// Incoming queue of scheduling requests.
    #[pin]
    requests: Fuse<R>,
}

impl<T, R: Stream> Scheduler<T, R> {
    fn new(requests: R) -> Self {
        Self {
            queue: DelayQueue::new(),
            scheduled: HashMap::new(),
            requests: requests.fuse(),
        }
    }
}

impl<T: Hash + Eq + Clone, R> SchedulerProj<'_, T, R> {
    /// Attempt to schedule a message into the queue.
    ///
    /// If the message is already in the queue then the earlier `request.run_at` takes precedence.
    fn schedule_message(&mut self, request: ScheduleRequest<T>) {
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
    ) -> Poll<Option<Result<delay_queue::Expired<T>, time::Error>>> {
        let message = self.queue.poll_expired(cx);
        if let Poll::Ready(Some(Ok(message))) = &message {
            self.scheduled.remove(message.get_ref()).expect(
                "Expired message was popped from the Scheduler queue, but was not in the metadata map",
            );
        }
        message
    }
}

impl<T, R> Stream for Scheduler<T, R>
where
    T: Eq + Hash + Clone,
    R: Stream<Item = ScheduleRequest<T>>,
{
    type Item = Result<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        while let Poll::Ready(Some(request)) = this.requests.as_mut().poll_next(cx) {
            this.schedule_message(request);
        }

        match this.poll_pop_queue_message(cx) {
            Poll::Ready(Some(expired)) => {
                Poll::Ready(Some(expired.map(Expired::into_inner).context(TimerError)))
            }
            Poll::Ready(None) => {
                if this.requests.is_terminated() {
                    // The source queue has terminated, and all outstanding requests are done, so terminate
                    Poll::Ready(None)
                } else {
                    // The delay queue is empty, empty, but we may get more requests in the future...
                    Poll::Pending
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Stream transformer that takes a message and `Instant` (in the form of a `ScheduleRequest`), and emits
/// the message at the specified `Instant`.
///
/// Objects are de-duplicated: if a message is submitted twice before being emitted then it will only be
/// emitted at the earlier of the two `Instant`s.
pub fn scheduler<T: Eq + Hash + Clone>(
    requests: impl Stream<Item = ScheduleRequest<T>>,
) -> impl Stream<Item = Result<T>> {
    Scheduler::new(requests)
}

#[cfg(test)]
mod tests {
    use super::{scheduler, ScheduleRequest};
    use futures::{channel::mpsc, poll, stream, FutureExt, SinkExt, StreamExt};
    use tokio::time::{advance, pause, Duration, Instant};

    #[tokio::test]
    async fn scheduler_should_emit_items_as_requested() {
        pause();
        let mut scheduler = scheduler(stream::iter(vec![
            ScheduleRequest {
                message: 1u8,
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
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), ());
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
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), ());
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
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), ());
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
        assert_eq!(scheduler.next().now_or_never().unwrap().unwrap().unwrap(), ());
        assert!(poll!(scheduler.next()).is_pending());
    }
}
