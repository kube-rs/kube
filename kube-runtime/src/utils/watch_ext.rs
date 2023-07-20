#[cfg(feature = "unstable-runtime-predicates")]
use crate::utils::predicate::{Predicate, PredicateFilter};
#[cfg(feature = "unstable-runtime-subscribe")]
use crate::utils::stream_subscribe::StreamSubscribe;
use crate::{
    utils::{event_flatten::EventFlatten, event_modify::EventModify, stream_backoff::StreamBackoff},
    watcher,
};
use kube_client::Resource;

use crate::{reflector::store::Writer, utils::Reflect};

use crate::watcher::DefaultBackoff;
use backoff::backoff::Backoff;
use futures::{Stream, TryStream};

/// Extension trait for streams returned by [`watcher`](watcher()) or [`reflector`](crate::reflector::reflector)
pub trait WatchStreamExt: Stream {
    /// Apply the [`DefaultBackoff`] watcher [`Backoff`] policy
    ///
    /// This is recommended for controllers that want to play nicely with the apiserver.
    fn default_backoff(self) -> StreamBackoff<Self, DefaultBackoff>
    where
        Self: TryStream + Sized,
    {
        StreamBackoff::new(self, DefaultBackoff::default())
    }

    /// Apply a specific [`Backoff`] policy to a [`Stream`] using [`StreamBackoff`]
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

    /// Modify elements of a [`watcher()`] stream.
    ///
    /// Calls [`watcher::Event::modify()`] on every element.
    /// Stream shorthand for `stream.map_ok(|event| { event.modify(f) })`.
    ///
    /// ```no_run
    /// # use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
    /// # use kube::{Api, Client, ResourceExt};
    /// # use kube_runtime::{watcher, WatchStreamExt};
    /// # use k8s_openapi::api::apps::v1::Deployment;
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let deploys: Api<Deployment> = Api::all(client);
    /// let truncated_deploy_stream = watcher(deploys, watcher::Config::default())
    ///     .modify(|deploy| {
    ///         deploy.managed_fields_mut().clear();
    ///         deploy.status = None;
    ///     })
    ///     .applied_objects();
    /// pin_mut!(truncated_deploy_stream);
    ///
    /// while let Some(d) = truncated_deploy_stream.try_next().await? {
    ///    println!("Truncated Deployment: '{:?}'", serde_json::to_string(&d)?);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn modify<F, K>(self, f: F) -> EventModify<Self, F>
    where
        Self: Stream<Item = Result<watcher::Event<K>, watcher::Error>> + Sized,
        F: FnMut(&mut K),
    {
        EventModify::new(self, f)
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
    /// use k8s_openapi::api::apps::v1::Deployment;
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let deploys: Api<Deployment> = Api::default_namespaced(client);
    /// let changed_deploys = watcher(deploys, watcher::Config::default())
    ///     .applied_objects()
    ///     .predicate_filter(predicates::generation);
    /// pin_mut!(changed_deploys);
    ///
    /// while let Some(d) = changed_deploys.try_next().await? {
    ///    println!("saw Deployment '{} with hitherto unseen generation", d.name_any());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "unstable-runtime-predicates")]
    fn predicate_filter<K, P>(self, predicate: P) -> PredicateFilter<Self, K, P>
    where
        Self: Stream<Item = Result<K, watcher::Error>> + Sized,
        K: Resource + 'static,
        P: Predicate<K> + 'static,
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

    /// Reflect a [`watcher()`] stream into a [`Store`] through a [`Writer`]
    ///
    /// Returns the stream unmodified, but passes every [`watcher::Event`] through a [`Writer`].
    /// This populates a [`Store`] as the stream is polled.
    ///
    /// ## Usage
    /// ```no_run
    /// # use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
    /// # use std::time::Duration;
    /// # use tracing::{info, warn};
    /// use kube::{Api, Client, ResourceExt};
    /// use kube_runtime::{watcher, WatchStreamExt, reflector};
    /// use k8s_openapi::api::apps::v1::Deployment;
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    ///
    /// let deploys: Api<Deployment> = Api::default_namespaced(client);
    /// let (reader, writer) = reflector::store::<Deployment>();
    ///
    /// tokio::spawn(async move {
    ///     // start polling the store once the reader is ready
    ///     reader.wait_until_ready().await.unwrap();
    ///     loop {
    ///         let names = reader.state().iter().map(|d| d.name_any()).collect::<Vec<_>>();
    ///         info!("Current {} deploys: {:?}", names.len(), names);
    ///         tokio::time::sleep(Duration::from_secs(10)).await;
    ///     }
    /// });
    ///
    /// // configure the watcher stream and populate the store while polling
    /// watcher(deploys, watcher::Config::default())
    ///     .reflect(writer)
    ///     .applied_objects()
    ///     .for_each(|res| async move {
    ///         match res {
    ///             Ok(o) => info!("saw {}", o.name_any()),
    ///             Err(e) => warn!("watcher error: {}", e),
    ///         }
    ///     })
    ///     .await;
    ///
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`Store`]: crate::reflector::Store
    fn reflect<K>(self, writer: Writer<K>) -> Reflect<Self, K>
    where
        Self: Stream<Item = watcher::Result<watcher::Event<K>>> + Sized,
        K: Resource + Clone + 'static,
        K::DynamicType: Eq + std::hash::Hash + Clone,
    {
        Reflect::new(self, writer)
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
