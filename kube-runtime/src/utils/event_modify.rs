use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::Arc;

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
    St: TryStream<Ok = Event<Arc<K>>>,
    F: FnMut(&mut K),
{
    pub(super) fn new(stream: St, f: F) -> EventModify<St, F> {
        Self { stream, f }
    }
}

impl<St, F, K> Stream for EventModify<St, F>
where
    St: Stream<Item = Result<Event<Arc<K>>, Error>>,
    F: FnMut(&mut K),
{
    type Item = Result<Event<Arc<K>>, Error>;

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
    use std::{sync::Arc, task::Poll, vec};

    use super::{Error, Event, EventModify};
    use futures::{pin_mut, poll, stream, StreamExt};

    #[tokio::test]
    async fn eventmodify_modifies_innner_value_of_event() {
        let st = stream::iter([
            Ok(Event::Applied(Arc::new(0))),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![Arc::new(10)])),
        ]);
        let ev_modify = EventModify::new(st, |x| {
            *x += 1;
        });
        pin_mut!(ev_modify);

        assert!(matches!(
            poll!(ev_modify.next()),
            Poll::Ready(Some(Ok(Event::Applied(x)))) if *x == 1
        ));

        assert!(matches!(
            poll!(ev_modify.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));

        let restarted = poll!(ev_modify.next());
        assert!(matches!(
            restarted,
            Poll::Ready(Some(Ok(Event::Restarted(vec)))) if vec.len() > 0
        ));

        assert!(matches!(poll!(ev_modify.next()), Poll::Ready(None)));
    }
}
