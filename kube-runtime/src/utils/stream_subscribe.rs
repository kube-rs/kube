use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{stream, Stream, TryStream};
use pin_project::pin_project;
use std::sync::Arc;
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
    sender: broadcast::Sender<Option<Arc<Result<S::Ok, S::Error>>>>,
}

impl<S: TryStream> StreamSubscribe<S> {
    pub fn new(stream: S) -> Self {
        let (sender, _) = broadcast::channel(100);

        Self { stream, sender }
    }

    /// Subscribe to events from this stream
    pub fn subscribe(&self) -> impl Stream<Item = Arc<Result<S::Ok, S::Error>>> {
        stream::unfold(self.sender.subscribe(), |mut rx| async move {
            match rx.recv().await {
                Ok(Some(obj)) => Some((obj, rx)),
                _ => None,
            }
        })
    }
}

impl<S: TryStream> Stream for StreamSubscribe<S> {
    type Item = Arc<Result<S::Ok, S::Error>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let item = this.stream.try_poll_next(cx);

        match item {
            Poll::Ready(Some(item)) => {
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
        let items = [Ok(0), Ok(1), Err(2), Ok(3), Ok(4)];
        let rx = stream::iter(items.clone());
        let rx = StreamSubscribe::new(rx);

        let rx_s1 = rx.subscribe();
        let rx_s2 = rx.subscribe();

        pin_mut!(rx);
        pin_mut!(rx_s1);
        pin_mut!(rx_s2);

        // Subscribers are pending until we start consuming the stream
        assert_eq!(poll!(rx_s1.next()), Poll::Pending, "rx_s1 - pending");
        assert_eq!(poll!(rx_s2.next()), Poll::Pending, "rx_s2 - pending");

        for (index, item) in items.into_iter().enumerate() {
            let expected = Poll::Ready(Some(Arc::new(item)));
            assert_eq!(poll!(rx.next()), expected, "rx - {}", index);
            assert_eq!(poll!(rx_s1.next()), expected, "rx_s1 - {}", index);
            assert_eq!(poll!(rx_s2.next()), expected, "rx_s2 - {}", index);
        }

        // Ensure that if the stream is closed, all subscribers are closed
        let expected = Poll::Ready(None);
        assert_eq!(poll!(rx.next()), expected, "rx - close");
        assert_eq!(poll!(rx_s1.next()), expected, "rx_s1 - close");
        assert_eq!(poll!(rx_s2.next()), expected, "rx_s2 - close");
    }
}
