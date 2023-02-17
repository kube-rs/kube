#[cfg(feature = "unstable_runtime_subscribe")]
use crate::utils::stream_subscribe::StreamSubscribe;
use crate::{
    utils::{event_flatten::EventFlatten, stream_backoff::StreamBackoff},
    watcher,
};
use backoff::backoff::Backoff;
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

    /// Create a [`StreamSubscribe`] from a [`watcher()`] stream.
    ///
    /// The [`StreamSubscribe::subscribe()`] method which allows additional consumers
    /// of events from a stream without consuming the stream itself.
    ///
    /// If a subscriber begins to lag behind the stream, it will receive an [`Error::Lagged`]
    /// error. The subscriber can then decide to abort its task or tolerate the lost events.
    ///
    /// If the [`Stream`] is dropped or ends, any [`StreamSubscribe::subscribe()`] streams
    /// will also end.
    ///
    /// ## Warning
    ///
    /// If the primary [`Stream`] is not polled, the [`StreamSubscribe::subscribe()`] streams
    /// will never receive any events.
    ///
    /// # Usage
    ///
    /// ```
    /// use futures::{Stream, StreamExt};
    /// use std::{fmt::Debug, sync::Arc};
    /// use kube_runtime::{watcher, WatchStreamExt};
    ///
    /// fn explain_events<K, S>(
    ///     stream: S,
    /// ) -> (
    ///     impl Stream<Item = Arc<Result<watcher::Event<K>, watcher::Error>>> + Send + Sized + 'static,
    ///     impl Stream<Item = String> + Send + Sized + 'static,
    /// )
    /// where
    ///     K: Debug + Send + Sync + 'static,
    ///     S: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Send + Sized + 'static,
    /// {
    ///     // Create a stream that can be subscribed to
    ///     let stream_subscribe = stream.stream_subscribe();
    ///     // Create a subscription to that stream
    ///     let subscription = stream_subscribe.subscribe();
    ///
    ///     // Create a stream of descriptions of the events
    ///     let explain_stream = subscription.filter_map(|event| async move {
    ///         // We don't care about lagged events so we can throw that error away
    ///         match event.ok()?.as_deref() {
    ///             Ok(watcher::Event::Applied(event)) => {
    ///                 Some(format!("An object was added or modified: {event:?}"))
    ///             }
    ///             Ok(_) => todo!("explain other events"),
    ///             // We don't care about watcher errors either
    ///             Err(_) => None,
    ///         }
    ///     });
    ///
    ///     // We now still have the original stream, and a secondary stream of explanations
    ///     (stream_subscribe, explain_stream)
    /// }
    /// ```
    #[cfg(feature = "unstable_runtime_subscribe")]
    fn stream_subscribe<K>(self) -> StreamSubscribe<Self>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Send + Sized + 'static,
    {
        StreamSubscribe::new(self)
    }
}

impl<St: ?Sized> WatchStreamExt for St where St: Stream {}
