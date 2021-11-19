use std::{pin::Pin, task::Poll};

use backoff::backoff::Backoff;
use futures::{Future, Stream, TryStream};
use pin_project::pin_project;
use tokio::time::{sleep, Instant, Sleep};

/// Applies a [`Backoff`] policy to a [`Stream`]
///
/// After any [`Err`] is emitted, the stream is paused for [`Backoff::next_backoff`]. The
/// [`Backoff`] is [`reset`](`Backoff::reset`) on any [`Ok`] value.
///
/// If [`Backoff::next_backoff`] returns [`None`] then the backing stream is given up on, and closed.
#[pin_project]
pub(crate) struct StreamBackoff<S, B> {
    #[pin]
    stream: S,
    backoff: B,
    #[pin]
    state: State,
}

#[pin_project(project = StreamBackoffStateProj)]
// It's expected to have relatively few but long-lived `StreamBackoff`s in a project, so we would rather have
// cheaper sleeps than a smaller `StreamBackoff`.
#[allow(clippy::large_enum_variant)]
enum State {
    BackingOff(#[pin] Sleep),
    GivenUp,
    Awake,
}

impl<S: TryStream, B: Backoff> StreamBackoff<S, B> {
    pub(crate) fn new(stream: S, backoff: B) -> Self {
        Self {
            stream,
            backoff,
            state: State::Awake,
        }
    }
}

impl<S: TryStream, B: Backoff> Stream for StreamBackoff<S, B> {
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match this.state.as_mut().project() {
            StreamBackoffStateProj::BackingOff(mut backoff_sleep) => match backoff_sleep.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    tracing::debug!(deadline = ?backoff_sleep.deadline(), "Backoff complete, waking up");
                    this.state.set(State::Awake)
                }
                Poll::Pending => {
                    let deadline = backoff_sleep.deadline();
                    tracing::trace!(
                        ?deadline,
                        remaining_duration = ?deadline.saturating_duration_since(Instant::now()),
                        "Still waiting for backoff sleep to complete"
                    );
                    return Poll::Pending;
                }
            },
            StreamBackoffStateProj::GivenUp => {
                tracing::debug!("Backoff has given up, stream is closed");
                return Poll::Ready(None);
            }
            StreamBackoffStateProj::Awake => {}
        }

        let next_item = this.stream.try_poll_next(cx);
        if let Poll::Ready(Some(Err(_))) = &next_item {
            if let Some(backoff_duration) = this.backoff.next_backoff() {
                let backoff_sleep = sleep(backoff_duration);
                tracing::debug!(
                    deadline = ?backoff_sleep.deadline(),
                    duration = ?backoff_duration,
                    "Error received, backing off"
                );
                this.state.set(State::BackingOff(backoff_sleep));
            } else {
                tracing::debug!("Error received, giving up");
                this.state.set(State::GivenUp);
            }
        } else {
            tracing::trace!("Non-error received, resetting backoff");
            this.backoff.reset();
        }
        next_item
    }
}
