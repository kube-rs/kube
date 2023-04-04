#[cfg(feature = "unstable-runtime-predicates")]
use crate::utils::predicate::PredicateFilter;
#[cfg(feature = "unstable-runtime-subscribe")]
use crate::utils::stream_subscribe::StreamSubscribe;
use crate::{
    utils::{event_flatten::EventFlatten, stream_backoff::StreamBackoff},
    watcher,
};
#[cfg(feature = "unstable-runtime-predicates")] use kube_client::Resource;

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


    /// Filter out a flattened stream on [`predicates`](crate::predicates).
    ///
    /// This will filter out repeat calls where the predicate returns the same result.
    /// Common use case for this is to avoid repeat events for status updates
    /// by filtering on [`predicates::generation`](crate::predicates::generation).
    ///
    /// **NB**: This is constructor requires an [`unstable`](https://github.com/kube-rs/kube/blob/main/kube-runtime/Cargo.toml#L17-L21) feature.
    ///
    /// ## Usage
    /// ```no_run
    /// # use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
    /// use kube::{Api, Client, ResourceExt};
    /// use kube_runtime::{watcher, WatchStreamExt, predicates};
    /// use k8s_openapi::api::core::v1::Pod;
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let pods: Api<Pod> = Api::default_namespaced(client);
    /// let changed_pods = watcher(pods, watcher::Config::default())
    ///     .applied_objects()
    ///     .predicate_filter(predicates::generation);
    /// pin_mut!(changed_pods);
    ///
    /// while let Some(pod) = changed_pods.try_next().await? {
    ///    println!("saw Pod '{} with hitherto unseen generation", pod.name_any());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "unstable-runtime-predicates")]
    fn predicate_filter<K, F>(self, predicate: F) -> PredicateFilter<Self, K, F>
    where
        Self: Stream<Item = Result<K, watcher::Error>> + Sized,
        K: Resource + 'static,
        F: Fn(&K) -> Option<u64> + 'static,
    {
        PredicateFilter::new(self, predicate)
    }

    /// Create a [`StreamSubscribe`] from a [`watcher()`] stream.
    ///
    /// The [`StreamSubscribe::subscribe()`] method which allows additional consumers
    /// of events from a stream without consuming the stream itself.
    ///
    /// If a subscriber begins to lag behind the stream, it will receive an [`Error::Lagged`](crate::utils::stream_subscribe::Error::Lagged)
    /// error. The subscriber can then decide to abort its task or tolerate the lost events.
    ///
    /// If the [`Stream`] is dropped or ends, any [`StreamSubscribe::subscribe()`] streams
    /// will also end.
    ///
    /// **NB**: This is constructor requires an [`unstable`](https://github.com/kube-rs/kube/blob/main/kube-runtime/Cargo.toml#L17-L21) feature.
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
    ///         match event.ok()?.as_ref() {
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
    #[cfg(feature = "unstable-runtime-subscribe")]
    fn stream_subscribe<K>(self) -> StreamSubscribe<Self>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Send + Sized + 'static,
    {
        StreamSubscribe::new(self)
    }
}

impl<St: ?Sized> WatchStreamExt for St where St: Stream {}

// Compile tests
#[cfg(feature = "unstable-runtime-predicates")]
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::predicates;
    use futures::StreamExt;
    use k8s_openapi::api::core::v1::Pod;
    use kube_client::{Api, Resource};

    fn compile_type<T>() -> T {
        unimplemented!("not called - compile test only")
    }

    pub fn assert_stream<T, K>(x: T) -> T
    where
        T: Stream<Item = watcher::Result<K>> + Send,
        K: Resource + Clone + Send + 'static,
    {
        x
    }

    // not #[test] because this is only a compile check verification
    #[allow(dead_code, unused_must_use)]
    fn test_watcher_stream_type_drift() {
        let pred_watch = watcher(compile_type::<Api<Pod>>(), Default::default())
            .touched_objects()
            .predicate_filter(predicates::generation)
            .boxed();
        assert_stream(pred_watch);
    }
}
