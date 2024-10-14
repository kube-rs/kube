use std::time::{Duration, Instant};

use backoff::{backoff::Backoff, Clock, SystemClock};

/// A [`Backoff`] wrapper that resets after a fixed duration has elapsed.
pub struct ResetTimerBackoff<B, C = SystemClock> {
    backoff: B,
    clock: C,
    last_backoff: Option<Instant>,
    reset_duration: Duration,
}

impl<B: Backoff> ResetTimerBackoff<B> {
    pub fn new(backoff: B, reset_duration: Duration) -> Self {
        Self::new_with_custom_clock(backoff, reset_duration, SystemClock {})
    }
}

impl<B: Backoff, C: Clock> ResetTimerBackoff<B, C> {
    fn new_with_custom_clock(backoff: B, reset_duration: Duration, clock: C) -> Self {
        Self {
            backoff,
            clock,
            last_backoff: None,
            reset_duration,
        }
    }
}

impl<B: Backoff, C: Clock> Backoff for ResetTimerBackoff<B, C> {
    fn next_backoff(&mut self) -> Option<Duration> {
        if let Some(last_backoff) = self.last_backoff {
            if self.clock.now() > last_backoff + self.reset_duration {
                tracing::debug!(
                    ?last_backoff,
                    reset_duration = ?self.reset_duration,
                    "Resetting backoff, since reset duration has expired"
                );
                self.backoff.reset();
            }
        }
        self.last_backoff = Some(self.clock.now());
        self.backoff.next_backoff()
    }

    fn reset(&mut self) {
        // Do not even bother trying to reset here, since `next_backoff` will take care of this when the timer expires.
    }
}

#[cfg(test)]
mod tests {
    use backoff::{backoff::Backoff, Clock};
    use tokio::time::advance;

    use super::ResetTimerBackoff;
    use crate::utils::stream_backoff::tests::LinearBackoff;
    use std::time::{Duration, Instant};

    #[tokio::test]
    async fn should_reset_when_timer_expires() {
        tokio::time::pause();
        let mut backoff = ResetTimerBackoff::new_with_custom_clock(
            LinearBackoff::new(Duration::from_secs(2)),
            Duration::from_secs(60),
            TokioClock,
        );
        assert_eq!(backoff.next_backoff(), Some(Duration::from_secs(2)));
        advance(Duration::from_secs(40)).await;
        assert_eq!(backoff.next_backoff(), Some(Duration::from_secs(4)));
        advance(Duration::from_secs(40)).await;
        assert_eq!(backoff.next_backoff(), Some(Duration::from_secs(6)));
        advance(Duration::from_secs(80)).await;
        assert_eq!(backoff.next_backoff(), Some(Duration::from_secs(2)));
        advance(Duration::from_secs(80)).await;
        assert_eq!(backoff.next_backoff(), Some(Duration::from_secs(2)));
    }

    struct TokioClock;

    impl Clock for TokioClock {
        fn now(&self) -> Instant {
            tokio::time::Instant::now().into_std()
        }
    }
}
