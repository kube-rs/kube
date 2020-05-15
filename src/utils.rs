use crate::watcher;
use futures::pin_mut;
use futures::{
    stream::{self, Peekable},
    Future, Stream, StreamExt, TryStream, TryStreamExt,
};
use pin_cell::{PinCell, PinMut};
use pin_project::pin_project;
use std::{fmt::Debug, pin::Pin, rc::Rc, task::Poll};
use stream::IntoStream;

/// Flattens each item in the list following the rules of `WatcherEvent::into_iter_added`
pub fn try_flatten_addeds<K, S: TryStream<Ok = watcher::Event<K>>>(
    stream: S,
) -> impl Stream<Item = Result<K, S::Error>> {
    stream
        .map_ok(|event| stream::iter(event.into_iter_added().map(Ok)))
        .try_flatten()
}

/// Flattens each item in the list following the rules of `WatcherEvent::into_iter_touched`
pub fn try_flatten_toucheds<K, S: TryStream<Ok = watcher::Event<K>>>(
    stream: S,
) -> impl Stream<Item = Result<K, S::Error>> {
    stream
        .map_ok(|event| stream::iter(event.into_iter_touched().map(Ok)))
        .try_flatten()
}

// #[pin_project]
// struct TryViaStream<Src, Via> {
//     input_stream: Src,
//     via_stream: Via,
// }

// impl<Src, Via> Stream for TryViaStream<Src, Via> {

// }

#[pin_project]
pub struct SplitResultOk<S: TryStream> {
    inner: Pin<Rc<PinCell<Peekable<IntoStream<S>>>>>,
}

impl<S> Stream for SplitResultOk<S>
where
    S: TryStream,
    S::Ok: Debug,
    S::Error: Debug,
{
    type Item = S::Ok;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let mut inner = PinCell::borrow_mut(this.inner.as_ref());
        let inner_peek = PinMut::as_mut(&mut inner).peek();
        pin_mut!(inner_peek);
        match inner_peek.poll(cx) {
            Poll::Ready(Some(Ok(_))) => match PinMut::as_mut(&mut inner).poll_next(cx) {
                Poll::Ready(Some(Ok(x))) => Poll::Ready(Some(x)),
                res => panic!("Peekable::poll_next() returned {:?} when Peekable::peek() returned Ready(Some(Ok(_)))", res)
            },
            // Err case will be handled by `SplitResultErr`
            Poll::Ready(Some(Err(_))) => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[pin_project]
struct SplitResultErr<S: TryStream> {
    inner: Pin<Rc<PinCell<Peekable<IntoStream<S>>>>>,
}

impl<S> Stream for SplitResultErr<S>
where
    S: TryStream,
    S::Ok: Debug,
    S::Error: Debug,
{
    type Item = S::Error;
    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let mut inner = PinCell::borrow_mut(this.inner.as_ref());
        let inner_peek = PinMut::as_mut(&mut inner).peek();
        pin_mut!(inner_peek);
        match inner_peek.poll(cx) {
            Poll::Ready(Some(Err(_))) => match PinMut::as_mut(&mut inner).poll_next(cx) {
                Poll::Ready(Some(Err(x))) => Poll::Ready(Some(x)),
                res => panic!("Peekable::poll_next() returned {:?} when Peekable::peek() returned Ready(Some(Error(_)))", res)
            },
            // Ok case will be handled by `SplitResultOk`
            Poll::Ready(Some(Ok(_))) => Poll::Pending,
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Splits a `TryStream` into separate `Ok` and `Error` streams.
///
/// Note: This will deadlock if one branch outlives the other
fn trystream_split_result<S>(stream: S) -> (SplitResultOk<S>, SplitResultErr<S>)
where
    S: TryStream,
    S::Ok: Debug,
    S::Error: Debug,
{
    let stream = Rc::pin(PinCell::new(stream.into_stream().peekable()));
    (
        SplitResultOk {
            inner: stream.clone(),
        },
        SplitResultErr { inner: stream },
    )
}

/// Forwards Ok elements via a stream built from `make_via_stream`, while passing errors through unmodified
pub fn trystream_try_via<S1, S2>(
    input_stream: S1,
    make_via_stream: impl FnOnce(SplitResultOk<S1>) -> S2,
) -> impl Stream<Item = Result<S2::Ok, S1::Error>>
where
    S1: TryStream,
    S2: TryStream<Error = S1::Error>,
    S1::Ok: Debug,
    S1::Error: Debug,
{
    let (oks, errs) = trystream_split_result(input_stream);
    let via = make_via_stream(oks);
    stream::select(via.into_stream(), errs.map(Err))
}
