use std::time::{Duration, Instant};

use backon::BackoffBuilder;

use super::{ResettableBackoff, ResettableBackoffWrapper};

/// Builder for [`ResetTimerBackoff`].
#[derive(Debug, Clone)]
pub struct ResetTimerBackoffBuilder<B> {
    inner_backoff_builder: B,
    reset_duration: Duration,
}

impl<B: BackoffBuilder> ResetTimerBackoffBuilder<B> {
    pub fn new(inner_backoff_builder: B, reset_duration: Duration) -> Self {
        Self {
            inner_backoff_builder,
            reset_duration,
        }
    }
}

impl<B: BackoffBuilder + Clone> BackoffBuilder for ResetTimerBackoffBuilder<B> {
    type Backoff = ResetTimerBackoff<ResettableBackoffWrapper<B>>;

    fn build(self) -> Self::Backoff {
        ResetTimerBackoff {
            inner_backoff: ResettableBackoffWrapper::new(self.inner_backoff_builder),
            reset_duration: self.reset_duration,
            last_backoff: None,
        }
    }
}


/// Wraps a [`Backoff`] and resets it after a fixed duration of inactivity has elapsed.
///
/// Constructed by [`ResetTimerBackoffBuilder`].
#[derive(Debug)]
pub struct ResetTimerBackoff<B> {
    inner_backoff: B,
    reset_duration: Duration,
    last_backoff: Option<Instant>,
}

// impl Backoff, which is now effectively an alias for Iterator<Item = Duration>
impl<B: ResettableBackoff> Iterator for ResetTimerBackoff<B> {
    type Item = Duration;

    fn next(&mut self) -> Option<Duration> {
        let now = tokio::time::Instant::now().into_std();
        if let Some(last_backoff) = self.last_backoff {
            if now > last_backoff + self.reset_duration {
                tracing::debug!(
                    ?last_backoff,
                    reset_duration = ?self.reset_duration,
                    "Resetting backoff, since reset duration has expired"
                );
                self.inner_backoff.reset();
            }
        }
        self.last_backoff = Some(now);
        self.inner_backoff.next()
    }
}

#[cfg(test)]
mod tests {
    use backon::BackoffBuilder;
    use tokio::time::advance;

    use crate::utils::{stream_backoff::tests::LinearBackoffBuilder, ResetTimerBackoffBuilder};
    use std::time::Duration;

    #[tokio::test]
    async fn should_reset_when_timer_expires() {
        tokio::time::pause();
        let mut backoff = ResetTimerBackoffBuilder::new(
            LinearBackoffBuilder::new(Duration::from_secs(2)),
            Duration::from_secs(60),
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
