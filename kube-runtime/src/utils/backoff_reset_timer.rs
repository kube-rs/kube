use std::time::{Duration, Instant};

use backon::BackoffBuilder;

use super::{ResettableBackoff, ResettableBackoffWrapper};

// TODO: do we actually need this or should we just use tokio::time?
pub trait Clock: Send + Sync + Unpin {
    fn now(&self) -> Instant;
}

#[derive(Debug, Clone, Copy)]
pub struct TokioClock;
impl Clock for TokioClock {
    fn now(&self) -> Instant {
        tokio::time::Instant::now().into_std()
    }
}

impl<B: BackoffBuilder> ResetTimerBackoffBuilder<B> {
    pub fn new(inner_backoff_builder: B, reset_duration: Duration) -> Self {
        Self::new_with_custom_clock(inner_backoff_builder, reset_duration, TokioClock)
    }
}

impl<B: BackoffBuilder, C: Clock> ResetTimerBackoffBuilder<B, C> {
    fn new_with_custom_clock(inner_backoff_builder: B, reset_duration: Duration, clock: C) -> Self {
        Self {
            inner_backoff_builder,
            clock,
            reset_duration,
        }
    }
}

/// A [`Backoff`] wrapper that resets after a fixed duration has elapsed.
#[derive(Debug, Clone)]
pub struct ResetTimerBackoffBuilder<B, C = TokioClock> {
    inner_backoff_builder: B,
    clock: C,
    reset_duration: Duration,
}

impl<B: BackoffBuilder + Clone, C: Clock> BackoffBuilder for ResetTimerBackoffBuilder<B, C> {
    type Backoff = ResetTimerBackoff<ResettableBackoffWrapper<B>, C>;

    fn build(self) -> Self::Backoff {
        ResetTimerBackoff {
            inner_backoff: ResettableBackoffWrapper::new(self.inner_backoff_builder),
            clock: self.clock,
            reset_duration: self.reset_duration,
            last_backoff: None,
        }
    }
}

/// Constructed by [`ResetTimerBackoffBuilder`].
#[derive(Debug)]
pub struct ResetTimerBackoff<B, C = TokioClock> {
    inner_backoff: B,
    clock: C,
    reset_duration: Duration,
    last_backoff: Option<Instant>,
}

// impl Backoff, which is now effectively an alias for Iterator<Item = Duration>
impl<B: ResettableBackoff, C: Clock> Iterator for ResetTimerBackoff<B, C> {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        if let Some(last_backoff) = self.last_backoff {
            if self.clock.now() > last_backoff + self.reset_duration {
                tracing::debug!(
                    ?last_backoff,
                    reset_duration = ?self.reset_duration,
                    "Resetting backoff, since reset duration has expired"
                );
                self.inner_backoff.reset();
            }
        }
        self.last_backoff = Some(self.clock.now());
        self.inner_backoff.next()
    }
}

#[cfg(test)]
mod tests {
    use backon::BackoffBuilder;
    use tokio::time::advance;

    use crate::utils::{
        backoff_reset_timer::TokioClock, stream_backoff::tests::LinearBackoffBuilder,
        ResetTimerBackoffBuilder,
    };
    use std::time::Duration;

    #[tokio::test]
    async fn should_reset_when_timer_expires() {
        tokio::time::pause();
        let mut backoff = ResetTimerBackoffBuilder::new_with_custom_clock(
            LinearBackoffBuilder::new(Duration::from_secs(2)),
            Duration::from_secs(60),
            TokioClock,
        )
        .build();
        assert_eq!(backoff.next(), Some(Duration::from_secs(2)));
        advance(Duration::from_secs(40)).await;
        assert_eq!(backoff.next(), Some(Duration::from_secs(4)));
        advance(Duration::from_secs(40)).await;
        assert_eq!(backoff.next(), Some(Duration::from_secs(6)));
        advance(Duration::from_secs(80)).await;
        assert_eq!(backoff.next(), Some(Duration::from_secs(2)));
        advance(Duration::from_secs(80)).await;
        assert_eq!(backoff.next(), Some(Duration::from_secs(2)));
    }
}
