use crate::{
    utils::{event_flatten::EventFlatten, stream_backoff::StreamBackoff},
    watcher,
};
use backoff::backoff::Backoff;

use futures::{Stream, TryStream};

/// Extension trait for streams returned by [`watcher`] or [`reflector`]
pub trait WatchStreamExt: Stream {
    /// Apply a [`Backoff`] policy to a [`Stream`] using [`StreamBackoff`]
    fn backoff<B>(self, b: B) -> StreamBackoff<Self, B>
    where
        B: Backoff,
        Self: TryStream + Sized,
    {
        StreamBackoff::new(self, b)
    }

    /// Flatten a [`watcher`] stream into a stream of applied objects
    ///
    /// All Added/Modified events are passed through, and critical errors bubble up.
    /// This is functionally equivalent to calling [`try_flatten_applied`] on a [`watcher`].
    fn applied_objects<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, false)
    }

    /// Flatten a [`watcher`] stream into a stream of touched objects
    ///
    /// All Added/Modified/Deleted events are passed through, and critical errors bubble up.
    /// This is functionally equivalent to calling [`try_flatten_touched`] on a [`watcher`].
    fn touched_objects<K>(self) -> EventFlatten<Self, K>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
    {
        EventFlatten::new(self, true)
    }
}
impl<St: ?Sized> WatchStreamExt for St where St: Stream {}
