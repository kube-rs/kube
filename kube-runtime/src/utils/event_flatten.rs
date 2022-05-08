use crate::watcher::{Error, Event};
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use futures::{Stream, TryStream};
use pin_project::pin_project;

// grab from private part of tokio
macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            std::task::Poll::Ready(t) => t,
            std::task::Poll::Pending => return std::task::Poll::Pending,
        }
    };
}

#[pin_project]
/// Stream returned by the [`watch_applies`](super::WatchStreamExt::watch_applies) and [`watch_touches`](super::WatchStreamExt::watch_touches) method.
#[must_use = "streams do nothing unless polled"]
pub struct EventFlatten<St, K> {
    #[pin]
    stream: St,
    delete: bool,
    state: Option<Result<Event<K>, Error>>,
}
impl<St: TryStream<Ok = Event<K>>, K> EventFlatten<St, K> {
    pub(super) fn new(stream: St, delete: bool) -> Self {
        Self {
            stream,
            state: None,
            delete,
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
        loop {
            if let Some(curr) = me.state.take() {
                match curr {
                    Ok(event) => {
                        // drain an individual event as per Event::into_iter_applied
                        match event {
                            Event::Applied(obj) => {
                                return Poll::Ready(Some(Ok(obj)));
                            }
                            Event::Deleted(obj) => {
                                // only pass delete events for touches
                                if *me.delete {
                                    return Poll::Ready(Some(Ok(obj)));
                                }
                            }
                            Event::Restarted(mut reslist) => {
                                if let Some(last) = reslist.pop() {
                                    // store the remainder
                                    *me.state = Some(Ok(Event::Restarted(reslist)));
                                    return Poll::Ready(Some(Ok(last)));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        return Poll::Ready(Some(Err(e)));
                    }
                }
            }
            let next = ready!(me.stream.as_mut().poll_next(cx));
            match next {
                Some(event) => {
                    *me.state = Some(event); // continue around loop to extract from it
                }
                None => return Poll::Pending,
            }
        }
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
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(1)))));
        // Error passed through
        assert!(matches!(
            poll!(rx.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        assert!(matches!(poll!(rx.next()), Poll::Ready(Some(Ok(2)))));
        assert!(matches!(poll!(rx.next()), Poll::Pending));
    }
}
