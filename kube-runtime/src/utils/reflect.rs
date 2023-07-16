use core::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, TryStream};
use pin_project::pin_project;

use crate::{
    reflector::store::Writer,
    watcher::{Error, Event},
};
use kube_client::Resource;

/// Stream returned by the [`reflect`](super::WatchStreamExt::reflect) method
#[pin_project]
pub struct Reflect<St, K>
where
    K: Resource + Clone + 'static,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    #[pin]
    stream: St,
    writer: Writer<K>,
}

impl<St, K> Reflect<St, K>
where
    St: TryStream<Ok = Event<K>>,
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
{
    pub(super) fn new(stream: St, writer: Writer<K>) -> Reflect<St, K> {
        Self { stream, writer }
    }
}

impl<St, K> Stream for Reflect<St, K>
where
    K: Resource + Clone,
    K::DynamicType: Eq + std::hash::Hash + Clone,
    St: Stream<Item = Result<Event<K>, Error>>,
{
    type Item = Result<Event<K>, Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut me = self.project();
        me.stream.as_mut().poll_next(cx).map_ok(move |event| {
            me.writer.apply_watcher_event(&event);
            event
        })
    }
}

#[cfg(test)]
pub(crate) mod test {
    use std::{task::Poll, vec};

    use super::{Error, Event, Reflect};
    use crate::reflector;
    use futures::{pin_mut, poll, stream, StreamExt};
    use k8s_openapi::api::core::v1::Pod;

    fn testpod(name: &str) -> Pod {
        let mut pod = Pod::default();
        pod.metadata.name = Some(name.to_string());
        pod
    }

    #[tokio::test]
    async fn reflect_passes_events_through() {
        let foo = testpod("foo");
        let bar = testpod("bar");
        let st = stream::iter([
            Ok(Event::Applied(foo.clone())),
            Err(Error::TooManyObjects),
            Ok(Event::Restarted(vec![foo, bar])),
        ]);
        let (reader, writer) = reflector::store();

        let reflect = Reflect::new(st, writer);
        pin_mut!(reflect);
        assert_eq!(reader.len(), 0);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Ok(Event::Applied(_))))
        ));
        assert_eq!(reader.len(), 1);

        assert!(matches!(
            poll!(reflect.next()),
            Poll::Ready(Some(Err(Error::TooManyObjects)))
        ));
        assert_eq!(reader.len(), 1);

        let restarted = poll!(reflect.next());
        assert!(matches!(restarted, Poll::Ready(Some(Ok(Event::Restarted(_))))));
        assert_eq!(reader.len(), 2);

        assert!(matches!(poll!(reflect.next()), Poll::Ready(None)));
        assert_eq!(reader.len(), 2);
    }
}
