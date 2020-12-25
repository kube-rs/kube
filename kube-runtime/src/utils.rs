use crate::watcher;
use futures::{
    future::{abortable, AbortHandle, Aborted},
    pin_mut,
    stream::{self, Peekable},
    Future, FutureExt, Stream, StreamExt, TryStream, TryStreamExt,
};
use pin_project::pin_project;
use std::{
    fmt::Debug,
    pin::Pin,
    sync::{Arc, Mutex},
    task::Poll,
};
use stream::IntoStream;
use tokio::{runtime::Handle, task::JoinHandle};

/// Flattens each item in the list following the rules of [`watcher::Event::into_iter_applied`].
pub fn try_flatten_applied<K, S: TryStream<Ok = watcher::Event<K>>>(
    stream: S,
) -> impl Stream<Item = Result<K, S::Error>> {
    stream
        .map_ok(|event| stream::iter(event.into_iter_applied().map(Ok)))
        .try_flatten()
}

/// Flattens each item in the list following the rules of [`watcher::Event::into_iter_touched`].
pub fn try_flatten_touched<K, S: TryStream<Ok = watcher::Event<K>>>(
    stream: S,
) -> impl Stream<Item = Result<K, S::Error>> {
    stream
        .map_ok(|event| stream::iter(event.into_iter_touched().map(Ok)))
        .try_flatten()
}

/// Allows splitting a `Stream` into several streams that each emit a disjoint subset of the input stream's items,
/// like a streaming variant of pattern matching.
///
/// NOTE: The cases MUST be reunited into the same final stream (using `futures::stream::select` or similar),
/// since cases for rejected items will *not* register wakeup correctly, and may otherwise lose items and/or deadlock.
///
/// NOTE: The whole set of cases will deadlock if there is ever an item that no live case wants to consume.
#[pin_project]
pub(crate) struct SplitCase<S: Stream, Case> {
    // Future-unaware `Mutex` is OK because it's only taken inside single poll()s
    inner: Arc<Mutex<Peekable<S>>>,
    /// Tests whether an item from the stream should be consumed
    ///
    /// NOTE: This MUST be total over all `SplitCase`s, otherwise the input stream
    /// will get stuck deadlocked because no candidate tries to consume the item.
    should_consume_item: fn(&S::Item) -> bool,
    /// Narrows the type of the consumed type, using the same precondition as `should_consume_item`.
    ///
    /// NOTE: This MUST return `Some` if `should_consume_item` returns `true`, since we can't put
    /// an item back into the input stream once consumed.
    try_extract_item_case: fn(S::Item) -> Option<Case>,
}

impl<S, Case> Stream for SplitCase<S, Case>
where
    S: Stream + Unpin,
    S::Item: Debug,
{
    type Item = Case;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        let mut inner = this.inner.lock().unwrap();
        let mut inner = Pin::new(&mut *inner);
        let inner_peek = inner.as_mut().peek();
        pin_mut!(inner_peek);
        match inner_peek.poll(cx) {
            Poll::Ready(Some(x_ref)) => {
                if (this.should_consume_item)(x_ref) {
                    match inner.as_mut().poll_next(cx) {
                        Poll::Ready(Some(x)) => Poll::Ready(Some((this.try_extract_item_case)(x).expect(
                            "`try_extract_item_case` returned `None` despite `should_consume_item` returning `true`",
                        ))),
                        res => panic!(
                    "Peekable::poll_next() returned {:?} when Peekable::peek() returned Ready(Some(_))",
                    res
                ),
                    }
                } else {
                    // Handled by another SplitCase instead
                    Poll::Pending
                }
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Splits a `TryStream` into separate `Ok` and `Error` streams.
///
/// Note: This will deadlock if one branch outlives the other
#[allow(clippy::type_complexity)]
fn trystream_split_result<S>(
    stream: S,
) -> (
    SplitCase<IntoStream<S>, S::Ok>,
    SplitCase<IntoStream<S>, S::Error>,
)
where
    S: TryStream + Unpin,
    S::Ok: Debug,
    S::Error: Debug,
{
    let stream = Arc::new(Mutex::new(stream.into_stream().peekable()));
    (
        SplitCase {
            inner: stream.clone(),
            should_consume_item: Result::is_ok,
            try_extract_item_case: Result::ok,
        },
        SplitCase {
            inner: stream,
            should_consume_item: Result::is_err,
            try_extract_item_case: Result::err,
        },
    )
}

/// Forwards Ok elements via a stream built from `make_via_stream`, while passing errors through unmodified
pub(crate) fn trystream_try_via<S1, S2>(
    input_stream: S1,
    make_via_stream: impl FnOnce(SplitCase<IntoStream<S1>, S1::Ok>) -> S2,
) -> impl Stream<Item = Result<S2::Ok, S1::Error>>
where
    S1: TryStream + Unpin,
    S2: TryStream<Error = S1::Error>,
    S1::Ok: Debug,
    S1::Error: Debug,
{
    let (oks, errs) = trystream_split_result(input_stream); // the select -> SplitCase
    let via = make_via_stream(oks); // the map_ok/err function
    stream::select(via.into_stream(), errs.map(Err)) // recombine
}

/// A [`JoinHandle`] that cancels the [`Future`] when dropped, rather than detaching it
pub struct CancelableJoinHandle<T> {
    inner: JoinHandle<Result<T, Aborted>>,
    abort_handle: AbortHandle,
}

impl<T> CancelableJoinHandle<T>
where
    T: Send + 'static,
{
    pub fn spawn(future: impl Future<Output = T> + Send + 'static, runtime: &Handle) -> Self {
        let (abortable, abort_handle) = abortable(future);
        CancelableJoinHandle {
            inner: runtime.spawn(abortable),
            abort_handle,
        }
    }
}

impl<T> Drop for CancelableJoinHandle<T> {
    fn drop(&mut self) {
        self.abort_handle.abort()
    }
}

impl<T> Future for CancelableJoinHandle<T> {
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.inner.poll_unpin(cx).map(|x| {
            x
                // JoinError => underlying future panicked, so propagate
                .unwrap()
                // Aborted => somehow the underlying future was aborted, which should only happen when the handle is dropped
                // (in which case poll would never be called again)
                .unwrap()
        })
    }
}
