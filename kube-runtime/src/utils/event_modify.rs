use core::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, TryStream};
use pin_project::pin_project;

use crate::watcher::{Error, Event};

#[pin_project]
/// Stream returned by the [`modify`](super::WatchStreamExt::modify) method.
/// Modifies the [`Event`] item returned by the inner stream by calling
/// [`modify`](Event::modify()) on it.
pub struct EventModify<St, F> {
    #[pin]
    stream: St,
    f: F,
}

impl<St, F, K> EventModify<St, F>
where
    St: TryStream<Ok = Event<K>>,
    F: FnMut(&mut K),
{
    pub(super) fn new(stream: St, f: F) -> EventModify<St, F> {
        Self { stream, f }
    }
}

impl<St, F, K> Stream for EventModify<St, F>
where
    St: Stream<Item = Result<Event<K>, Error>>,
    F: FnMut(&mut K),
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        me.stream
            .as_mut()
            .poll_next(cx)
            .map_ok(|event| event.modify(me.f))
    }
}

#[cfg(test)]
pub(crate) mod test {
    use std::{task::Poll, vec};

    use super::{Error, Event, EventModify};
    use futures::{pin_mut, poll, stream, StreamExt};

    #[tokio::test]
    async fn eventmodify_modifies_innner_value_of_event() {
        let st = stream::iter([
            Ok(Event::Applied(0)),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![10])),
        ]);
        let ev_modify = EventModify::new(st, |x| {
            *x += 1;
        });
        pin_mut!(ev_modify);

        assert!(matches!(
            poll!(ev_modify.next()),
            Poll::Ready(Some(Ok(Event::Applied(1))))
        ));

        assert!(matches!(
            poll!(ev_modify.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));

        let restarted = poll!(ev_modify.next());
        assert!(matches!(
            restarted,
            Poll::Ready(Some(Ok(Event::Restarted(vec)))) if vec == [11]
        ));

        assert!(matches!(poll!(ev_modify.next()), Poll::Ready(None)));
    }
}
