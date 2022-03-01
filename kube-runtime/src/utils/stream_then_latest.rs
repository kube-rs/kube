use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Future, Stream};
use pin_project::pin_project;

/// Computes new items from the latest values emitted by the stream, cancelling the current item if a new item is emitted
///
/// ```rust
/// use futures::stream;
///
/// let stream = stream::iter([1, 2, 3]);
/// let stream = ThenLatest::new(stream, |x| async move { x + 3 }});
///
/// assert_eq!(vec![6], stream.collect::<Vec<_>>().await);
/// ```
#[pin_project]
pub struct ThenLatest<S, F, Fut> {
    #[pin]
    stream: PinOption<S>,
    f: F,
    #[pin]
    current_fut: PinOption<Fut>,
}

#[pin_project(project = PinOptionProj)]
enum PinOption<T> {
    Some(#[pin] T),
    None,
}

impl<S, F, Fut> ThenLatest<S, F, Fut>
where
    S: Stream,
    F: FnMut(S::Item) -> Fut,
    Fut: Future,
{
    pub fn new(stream: S, f: F) -> Self {
        Self {
            stream: PinOption::Some(stream),
            f,
            current_fut: PinOption::None,
        }
    }
}

impl<S, F, Fut> Stream for ThenLatest<S, F, Fut>
where
    S: Stream,
    F: FnMut(S::Item) -> Fut,
    Fut: Future,
{
    type Item = Fut::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        if let PinOptionProj::Some(mut stream) = this.stream.as_mut().project() {
            while let Poll::Ready(item) = stream.as_mut().poll_next(cx) {
                if let Some(item) = item {
                    // Cancel the current item's future, start a new one for the new item
                    this.current_fut.set(PinOption::Some((this.f)(item)));
                } else {
                    // Backing stream is finished, so stop reading from it but still try to finish processing the current item (if any)
                    this.stream.set(PinOption::None);
                    break;
                }
            }
        }

        if let PinOptionProj::Some(current_fut) = this.current_fut.as_mut().project() {
            match current_fut.poll(cx) {
                Poll::Ready(output) => {
                    // Disable future, so that we don't poll it again after it has completed
                    this.current_fut.set(PinOption::None);
                    // Return the value!
                    Poll::Ready(Some(output))
                }
                // Pending while we wait for the current item's future to finish (or for a new item to be received)
                Poll::Pending => Poll::Pending,
            }
        } else {
            match this.stream.project() {
                // Pending because the stream is still open
                PinOptionProj::Some(_) => Poll::Pending,
                // The current item is done processing, and the stream is closed, we're done!
                PinOptionProj::None => Poll::Ready(None),
            }
        }
    }
}
