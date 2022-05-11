use crate::watcher::{Error, Event};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{ready, Stream, TryStream};
use pin_project::pin_project;

#[pin_project]
/// Stream returned by the [`applied_objects`](super::WatchStreamExt::applied_objects) and [`touched_objects`](super::WatchStreamExt::touched_objects) method.
#[must_use = "streams do nothing unless polled"]
pub struct EventFlatten<St, K> {
    #[pin]
    stream: St,
    emit_deleted: bool,
    queue: std::vec::IntoIter<K>,
}
impl<St: TryStream<Ok = Event<K>>, K> EventFlatten<St, K> {
    pub(super) fn new(stream: St, emit_deleted: bool) -> Self {
        Self {
            stream,
            queue: vec![].into_iter(),
            emit_deleted,
        }
    }
}
impl<St, K> Stream for EventFlatten<St, K>
where
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<K, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        Poll::Ready(loop {
            if let Some(item) = me.queue.next() {
                break Some(Ok(item));
            }
            break match ready!(me.stream.as_mut().poll_next(cx)) {
                Some(Ok(Event::Applied(obj))) => Some(Ok(obj)),
                Some(Ok(Event::Deleted(obj))) => {
                    if *me.emit_deleted {
                        Some(Ok(obj))
                    } else {
                        continue;
                    }
                }
                Some(Ok(Event::Restarted(objs))) => {
                    *me.queue = objs.into_iter();
                    continue;
                }
                Some(Err(err)) => Some(Err(err)),
                None => return Poll::Ready(None),
            };
        })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::task::Poll;

    use super::{Error, Event, EventFlatten};
    use futures::{pin_mut, poll, stream, StreamExt};

    #[tokio::test]
    async fn watches_applies_uses_correct_eventflattened_stream() {
        let data = stream::iter([
            Ok(Event::Applied(0)),
            Ok(Event::Applied(1)),
            Ok(Event::Deleted(0)),
            Ok(Event::Applied(2)),
            Ok(Event::Restarted(vec![1, 2])),
            Err(Error::TooManyObjects),
            Ok(Event::Applied(2)),
        ]);
        let rx = EventFlatten::new(data, false);
        pin_mut!(rx);
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
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        assert!(matches!(poll!(rx.next()), Poll::Ready(None)));
    }
}
