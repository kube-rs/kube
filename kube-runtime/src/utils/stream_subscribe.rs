use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{stream, Stream, TryStream};
use pin_project::pin_project;
use tokio::sync::broadcast;

/// Exposes the [`StreamSubscribe::subscribe_ok()`] method that allows additional
/// consumers of [`Ok`] events from a stream without consuming the stream itself.
///
/// If the [`Stream`] is dropped or ends, any [`StreamSubscribe::subscribe_ok()`] streams
/// will also end.
#[pin_project]
#[must_use = "streams do nothing unless polled"]
pub struct StreamSubscribe<S>
where
    S: TryStream,
{
    #[pin]
    stream: S,
    sender: broadcast::Sender<Option<S::Ok>>,
}

impl<S: TryStream> StreamSubscribe<S>
where
    S::Ok: Clone,
{
    pub fn new(stream: S) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self { stream, sender }
    }

    /// Subscribe to success events from this stream
    pub fn subscribe_ok(&self) -> impl Stream<Item = S::Ok> {
        stream::unfold(self.sender.subscribe(), |mut rx| async move {
            match rx.recv().await {
                Ok(Some(obj)) => Some((obj, rx)),
                _ => None,
            }
        })
    }
}

impl<S: TryStream> Stream for StreamSubscribe<S>
where
    S::Ok: Clone,
{
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let item = this.stream.try_poll_next(cx);

        if let Poll::Ready(Some(Ok(item))) = &item {
            this.sender.send(Some((*item).clone())).ok();
        } else if let Poll::Ready(None) = &item {
            // If the stream is closed, we need to send a None to all subscribers
            // which will cause them to close.
            this.sender.send(None).ok();
        }

        item
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{pin_mut, poll, stream, StreamExt};

    #[tokio::test]
    async fn stream_subscribe_continues_to_propagate_values() {
        let rx = stream::iter([Ok(0), Ok(1), Err(2), Ok(3), Ok(4)]);
        let rx = StreamSubscribe::new(rx);

        pin_mut!(rx);
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Ok(0))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Ok(1))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Err(2))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Ok(3))));
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Ok(4))));
        assert_eq!(poll!(rx.next()), Poll::Ready(None));
    }

    #[tokio::test]
    async fn all_subscribers_get_success_events() {
        let rx = stream::iter([Ok(0), Err(1)]);
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe_ok();
        let rx_s2 = rx.subscribe_ok();

        pin_mut!(rx);
        pin_mut!(rx_s1);
        pin_mut!(rx_s2);

        assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s1");
        assert_eq!(poll!(rx_s2.next()), Poll::Pending, "rx_s2");

        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Ok(0))), "rx");
        assert_eq!(poll!(rx_s1.next()), Poll::Ready(Some(0)), "rx_s1");
        assert_eq!(poll!(rx_s2.next()), Poll::Ready(Some(0)), "rx_s2");

        // Subscribers don't get error events
        assert_eq!(poll!(rx.next()), Poll::Ready(Some(Err(1))), "rx");
        assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s1");
        assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s2");

        // Ensure that if the stream is closed, all subscribers are closed
        assert_eq!(poll!(rx.next()), Poll::Ready(None), "rx");
        assert_eq!(poll!(rx_s1.next()), Poll::Ready(None), "rx_s1");
        assert_eq!(poll!(rx_s2.next()), Poll::Ready(None), "rx_s2");
    }
}
