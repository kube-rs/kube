use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project::{pin_project, project};
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

#[derive(Debug)]
pub struct ScheduleRequest<T> {
    pub message: T,
    pub run_at: Instant,
}

struct ScheduledEntry {
    run_at: Instant,
    queue_key: delay_queue::Key,
}

#[pin_project]
struct Scheduler<T, R> {
    queue: DelayQueue<T>,
    scheduled: HashMap<T, ScheduledEntry>,
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

#[project]
impl<T: Hash + Eq + Clone, R> Scheduler<T, R> {
    fn schedule_message(&mut self, request: ScheduleRequest<T>) {
        match self.scheduled.entry(request.message) {
            Entry::Occupied(mut old_entry) if old_entry.get().run_at >= request.run_at => {
                // Old entry will run after the new request, so replace it..
                let entry = old_entry.get_mut();
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

    fn poll_pop_queue_message(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<delay_queue::Expired<T>, time::Error>>> {
        let message = self.queue.poll_expired(cx);
        match &message {
            Poll::Ready(Some(Ok(message))) => {
                self.scheduled.remove(message.get_ref()).expect("Expired message was popped from the Scheduler queue, but was not in the metadata map");
            }
            _ => {}
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

/// Stream transformer that schedules each item to be performed at a given time.
pub fn scheduler<T: Eq + Hash + Clone>(
    requests: impl Stream<Item = ScheduleRequest<T>>,
) -> impl Stream<Item = Result<T>> {
    Scheduler::new(requests)
}
