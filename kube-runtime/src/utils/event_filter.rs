use crate::watcher::{Error, Event};
use core::{
    pin::Pin,
    task::{ready, Context, Poll},
};
use futures::{Stream, TryStream};
use pin_project::pin_project;

#[pin_project]
/// Stream returned by the [`applied_objects`](super::WatchStreamExt::applied_objects) and [`touched_objects`](super::WatchStreamExt::touched_objects) method.
#[must_use = "streams do nothing unless polled"]
pub struct EventDecode<St> {
    #[pin]
    stream: St,
    emit_deleted: bool,
}
impl<St: TryStream<Ok = Event<K>>, K> EventDecode<St> {
    pub(super) fn new(stream: St, emit_deleted: bool) -> Self {
        Self { stream, emit_deleted }
    }
}
impl<St, K> Stream for EventDecode<St>
where
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<K, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        Poll::Ready(loop {
            let var_name = match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(Event::Apply(obj) | Event::InitApply(obj))) => Some(Ok(obj)),
                Some(Ok(Event::Delete(obj))) => {
                    if *me.emit_deleted {
                        Some(Ok(obj))
                    } else {
                        continue;
                    }
                }
                Some(Ok(Event::Init | Event::InitDone)) => continue,
                Some(Err(err)) => Some(Err(err)),
                None => return Poll::Ready(None),
            };
            break var_name;
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{pin::pin, task::Poll};

    use super::{Error, Event, EventDecode};
    use futures::{poll, stream, StreamExt};

    #[tokio::test]
    async fn watches_applies_uses_correct_stream() {
        let data = stream::iter([
            Ok(Event::Apply(0)),
            Ok(Event::Apply(1)),
            Ok(Event::Delete(0)),
            Ok(Event::Apply(2)),
            Ok(Event::InitApply(1)),
            Ok(Event::InitApply(2)),
            Err(Error::NoResourceVersion),
            Ok(Event::Apply(2)),
        ]);
        let mut rx = pin!(EventDecode::new(data, false));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(0)))));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(1)))));
        // NB: no Deleted events here
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        // Restart comes through, currently in reverse order
        // (normally on restart they just come in alphabetical order by name)
        // this is fine though, alphabetical event order has no functional meaning in watchers
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(1)))));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        // Error passed through
        assert!(matches!(
            poll!(rx.next()),
            Poll::Ready(Some(Err(Error::NoResourceVersion)))
        ));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }
}
