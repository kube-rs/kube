use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{stream, Stream};
use pin_project::pin_project;
use std::{fmt, sync::Arc};
use tokio::sync::{broadcast, broadcast::error::RecvError};

const CHANNEL_CAPACITY: usize = 128;

/// Exposes the [`StreamSubscribe::subscribe()`] method which allows additional
/// consumers of events from a stream without consuming the stream itself.
///
/// If a subscriber begins to lag behind the stream, it will receive an [`Error::Lagged`]
/// error. The subscriber can then decide to abort its task or tolerate the lost events.
///
/// If the [`Stream`] is dropped or ends, any [`StreamSubscribe::subscribe()`] streams
/// will also end.
///
/// ## Warning
///
/// If the primary [`Stream`] is not polled, the [`StreamSubscribe::subscribe()`] streams
/// will never receive any events.
#[pin_project]
#[must_use = "subscribers will not get events unless this stream is polled"]
pub struct StreamSubscribe<S>
where
    S: Stream,
{
    #[pin]
    stream: S,
    sender: broadcast::Sender<Option<Arc<S::Item>>>,
}

impl<S: Stream> StreamSubscribe<S> {
    pub fn new(stream: S) -> Self {
        let (sender, _) = broadcast::channel(CHANNEL_CAPACITY);

        Self { stream, sender }
    }

    /// Subscribe to events from this stream
    #[must_use = "streams do nothing unless polled"]
    pub fn subscribe(&self) -> impl Stream<Item = Result<Arc<S::Item>, Error>> {
        stream::unfold(self.sender.subscribe(), |mut rx| async {
            match rx.recv().await {
                Ok(Some(obj)) => Some((Ok(obj), rx)),
                Err(RecvError::Lagged(amt)) => Some((Err(Error::Lagged(amt)), rx)),
                _ => None,
            }
        })
    }
}

impl<S: Stream> Stream for StreamSubscribe<S> {
    type Item = Arc<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let item = this.stream.poll_next(cx);

        match item {
            Poll::Ready(Some(item)) => {
                #[allow(clippy::arc_with_non_send_sync)]
                // ^ this whole module is unstable and does not have a PoC
                let item = Arc::new(item);
                this.sender.send(Some(item.clone())).ok();
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                this.sender.send(None).ok();
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// An error returned from the inner stream of a [`StreamSubscribe`].
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Error {
    /// The subscriber lagged too far behind. Polling again will return
    /// the oldest event still retained.
    ///
    /// Includes the number of skipped events.
    Lagged(u64),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lagged(amt) => write!(f, "subscriber lagged by {amt}"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{pin_mut, poll, stream, StreamExt};

    #[tokio::test]
    async fn stream_subscribe_continues_to_propagate_values() {
        let rx = stream::iter([Ok(0), Ok(1), Err(2), Ok(3), Ok(4)]);
        let rx = StreamSubscribe::new(rx);

        pin_mut!(rx);
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(Ok(0)))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(Ok(1)))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(Err(2)))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(Ok(3)))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(Ok(4)))));
        assert_eq!(poll!(rx.next()), Poll::Ready(None));
    }

    #[tokio::test]
    async fn all_subscribers_get_events() {
        let events = [Ok(0), Ok(1), Err(2), Ok(3), Ok(4)];
        let rx = stream::iter(events);
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe();
        let rx_s2 = rx.subscribe();

        pin_mut!(rx);
        pin_mut!(rx_s1);
        pin_mut!(rx_s2);

        // Subscribers are pending until we start consuming the stream
        assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s1");
        assert_eq!(poll!(rx_s2.next()), Poll::Pending, "rx_s2");

        for item in events {
            assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(item))), "rx");
            let expected = Poll::Ready(Some(Ok(Arc::new(item))));
            assert_eq!(poll!(rx_s1.next()), expected, "rx_s1");
            assert_eq!(poll!(rx_s2.next()), expected, "rx_s2");
        }

        // Ensure that if the stream is closed, all subscribers are closed
        assert_eq!(poll!(rx.next()), Poll::Ready(None), "rx");
        assert_eq!(poll!(rx_s1.next()), Poll::Ready(None), "rx_s1");
        assert_eq!(poll!(rx_s2.next()), Poll::Ready(None), "rx_s2");
    }

    #[tokio::test]
    async fn subscribers_can_catch_up_to_the_main_stream() {
        let events = (0..CHANNEL_CAPACITY).map(Ok::<_, ()>).collect::<Vec<_>>();
        let rx = stream::iter(events.clone());
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe();

        pin_mut!(rx);
        pin_mut!(rx_s1);

        for item in events.clone() {
            assert_eq!(poll!(rx.next()), Poll::Ready(Some(Arc::new(item))), "rx",);
        }

        for item in events {
            assert_eq!(
                poll!(rx_s1.next()),
                Poll::Ready(Some(Ok(Arc::new(item)))),
                "rx_s1"
            );
        }
    }

    #[tokio::test]
    async fn if_the_subscribers_lag_they_get_a_lagged_error_as_the_next_event() {
        // The broadcast channel rounds the capacity up to the next power of two.
        let max_capacity = CHANNEL_CAPACITY.next_power_of_two();
        let overflow = 5;
        let events = (0..max_capacity + overflow).collect::<Vec<_>>();
        let rx = stream::iter(events.clone());
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe();

        pin_mut!(rx);
        pin_mut!(rx_s1);

        // Consume the entire stream, overflowing the inner channel
        for _ in events {
            rx.next().await;
        }

        assert_eq!(
            poll!(rx_s1.next()),
            Poll::Ready(Some(Err(Error::Lagged(overflow as u64)))),
        );

        let expected_next_event = overflow;
        assert_eq!(
            poll!(rx_s1.next()),
            Poll::Ready(Some(Ok(Arc::new(expected_next_event)))),
        );
    }

    #[tokio::test]
    async fn a_lagging_subscriber_does_not_impact_a_well_behaved_subscriber() {
        // The broadcast channel rounds the capacity up to the next power of two.
        let max_capacity = CHANNEL_CAPACITY.next_power_of_two();
        let overflow = 5;
        let events = (0..max_capacity + overflow).collect::<Vec<_>>();
        let rx = stream::iter(events.clone());
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe();
        let rx_s2 = rx.subscribe();

        pin_mut!(rx);
        pin_mut!(rx_s1);
        pin_mut!(rx_s2);

        for event in events {
            assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s1");

            rx.next().await;

            assert_eq!(
                poll!(rx_s1.next()),
                Poll::Ready(Some(Ok(Arc::new(event)))),
                "rx_s1"
            );
        }

        assert_eq!(
            poll!(rx_s2.next()),
            Poll::Ready(Some(Err(Error::Lagged(overflow as u64)))),
            "rx_s2"
        );

        let expected_next_event = overflow;
        assert_eq!(
            poll!(rx_s2.next()),
            Poll::Ready(Some(Ok(Arc::new(expected_next_event)))),
            "rx_s2"
        );
    }
}
