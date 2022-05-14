use crate::{
    utils::{event_flatten::EventFlatten, predicate::PredicateFilter, stream_backoff::StreamBackoff},
    watcher,
};
use backoff::backoff::Backoff;
use kube_client::Resource;

use futures::{Stream, TryStream};

/// Extension trait for streams returned by [`watcher`](watcher()) or [`reflector`](crate::reflector::reflector)
pub trait WatchStreamExt: Stream {
    /// Apply a [`Backoff`] policy to a [`Stream`] using [`StreamBackoff`]
    fn backoff<B>(self, b: B) -> StreamBackoff<Self, B>
    where
        B: Backoff,
        Self: TryStream + Sized,
    {
        StreamBackoff::new(self, b)
    }

    /// Flatten a [`watcher()`] stream into a stream of applied objects
    ///
    /// All Added/Modified events are passed through, and critical errors bubble up.
    fn applied_objects<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, false)
    }

    /// Flatten a [`watcher()`] stream into a stream of touched objects
    ///
    /// All Added/Modified/Deleted events are passed through, and critical errors bubble up.
    fn touched_objects<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, true)
    }

    /// Filter out a flattened stream on predicates
    fn predicate_filter<K, V>(
        self,
        predicate: impl Fn(&K) -> Option<V> + 'static,
    ) -> PredicateFilter<Self, K, V>
    where
        Self: Stream<Item = Result<K, watcher::Error>> + Sized,
        V: PartialEq,
        K: Resource + 'static,
    {
        PredicateFilter::new(self, predicate)
    }
}
impl<St: ?Sized> WatchStreamExt for St where St: Stream {}
