use crate::{
    utils::{event_flatten::EventFlatten, stream_backoff::StreamBackoff},
    watcher,
};
use backoff::backoff::Backoff;

use crate::utils::stream_subscribe::StreamSubscribe;
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

    /// Create a [`StreamSubscribe`] from a [`watcher()`] stream
    ///
    /// Allows additional consumers to subscribe to the stream, without consuming the stream itself.
    fn subscribe<K>(self) -> StreamSubscribe<Self>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Send + Sized + 'static,
    {
        StreamSubscribe::new(self)
    }
}

impl<St: ?Sized> WatchStreamExt for St where St: Stream {}
