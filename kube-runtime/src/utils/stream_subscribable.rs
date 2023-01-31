use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{stream, Stream, TryFutureExt, TryStream};
use pin_project::pin_project;
use tokio::sync::broadcast;

#[pin_project]
/// todo - docs
#[must_use = "streams do nothing unless polled"]
pub struct StreamSubscribable<S>
where
    S: TryStream,
{
    #[pin]
    stream: S,
    sender: broadcast::Sender<S::Ok>,
}

impl<S: TryStream> StreamSubscribable<S>
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
                Ok(obj) => Some((obj, rx)),
                Err(_) => None,
            }
        })
    }
}

impl<S: TryStream> Stream for StreamSubscribable<S>
where
    S::Ok: Clone,
{
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let item = this.stream.try_poll_next(cx);

        if let Poll::Ready(Some(Ok(item))) = &item {
            this.sender.send((*item).clone()).ok();
        }

        item
    }
}
