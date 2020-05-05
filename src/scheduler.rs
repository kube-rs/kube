use futures::{
    stream::{Fuse, FusedStream},
    Stream, StreamExt,
};
use pin_project::pin_project;
use snafu::{Backtrace, ResultExt, Snafu};
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use time::delay_queue::Expired;
use tokio::time::{self, DelayQueue, Instant};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("timer failure: {}", source))]
    TimerError {
        source: time::Error,
        backtrace: Backtrace,
    },
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub struct ScheduleRequest<T> {
    pub message: T,
    pub run_at: Instant,
}

#[pin_project]
struct Scheduler<T, R> {
    queue: DelayQueue<T>,
    #[pin]
    requests: Fuse<R>,
}

impl<T, R: Stream> Scheduler<T, R> {
    fn new(requests: R) -> Self {
        Self {
            queue: DelayQueue::new(),
            requests: requests.fuse(),
        }
    }
}

impl<T, R> Stream for Scheduler<T, R>
where
    R: Stream<Item = ScheduleRequest<T>>,
{
    type Item = Result<T>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.as_mut().project();

        while let Poll::Ready(Some(request)) = this.requests.as_mut().poll_next(cx) {
            this.queue.insert_at(request.message, request.run_at);
        }

        match this.queue.poll_expired(cx) {
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
pub fn scheduler<T>(
    requests: impl Stream<Item = ScheduleRequest<T>>,
) -> impl Stream<Item = Result<T>> {
    Scheduler::new(requests)
}
