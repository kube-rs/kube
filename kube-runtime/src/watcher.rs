//! Watches a Kubernetes Resource for changes, with error recovery
//!
//! See [`watcher`] for the primary entry point.

use crate::utils::ResetTimerBackoff;
use backoff::{backoff::Backoff, ExponentialBackoff};
use derivative::Derivative;
use futures::{stream::BoxStream, Stream, StreamExt};
use kube_client::{
    api::{ListParams, Resource, ResourceExt, WatchEvent},
    Api,
};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use std::{clone::Clone, fmt::Debug, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to perform initial object list: {0}")]
    InitialListFailed(#[source] kube_client::Error),
    #[error("failed to start watching object: {0}")]
    WatchStartFailed(#[source] kube_client::Error),
    #[error("error returned by apiserver during watch: {0}")]
    WatchError(#[source] kube_client::error::ErrorResponse),
    #[error("watch stream failed: {0}")]
    WatchFailed(#[source] kube_client::Error),
    #[error("too many objects matched search criteria")]
    TooManyObjects,
}
pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Clone)]
/// Watch events returned from the [`watcher`]
pub enum Event<K> {
    /// An object was added or modified
    Applied(K),
    /// An object was deleted
    ///
    /// NOTE: This should not be used for managing persistent state elsewhere, since
    /// events may be lost if the watcher is unavailable. Use Finalizers instead.
    Deleted(K),
    /// The watch stream was restarted, so `Deleted` events may have been missed
    ///
    /// Should be used as a signal to replace the store contents atomically.
    ///
    /// Any objects that were previously [`Applied`](Event::Applied) but are not listed in this event
    /// should be assumed to have been [`Deleted`](Event::Deleted).
    Restarted(Vec<K>),
}

impl<K> Event<K> {
    /// Flattens out all objects that were added or modified in the event.
    ///
    /// `Deleted` objects are ignored, all objects mentioned by `Restarted` events are
    /// emitted individually.
    pub fn into_iter_applied(self) -> impl Iterator<Item = K> {
        match self {
            Event::Applied(obj) => SmallVec::from_buf([obj]),
            Event::Deleted(_) => SmallVec::new(),
            Event::Restarted(objs) => SmallVec::from_vec(objs),
        }
        .into_iter()
    }

    /// Flattens out all objects that were added, modified, or deleted in the event.
    ///
    /// Note that `Deleted` events may be missed when restarting the stream. Use finalizers
    /// or owner references instead if you care about cleaning up external resources after
    /// deleted objects.
    pub fn into_iter_touched(self) -> impl Iterator<Item = K> {
        match self {
            Event::Applied(obj) | Event::Deleted(obj) => SmallVec::from_buf([obj]),
            Event::Restarted(objs) => SmallVec::from_vec(objs),
        }
        .into_iter()
    }

    /// Map each object in an event through a mutator fn
    ///
    /// This allows for memory optimizations in watch streams.
    /// If you are chaining a watch stream into a reflector as an in memory state store,
    /// you can control the space used by each object by dropping fields.
    ///
    /// ```no_run
    /// use k8s_openapi::api::core::v1::Pod;
    /// use kube::ResourceExt;
    /// # use kube::runtime::watcher::Event;
    /// # let event: Event<Pod> = todo!();
    /// event.modify(|pod| {
    ///     pod.managed_fields_mut().clear();
    ///     pod.annotations_mut().clear();
    ///     pod.status = None;
    /// });
    /// ```
    #[must_use]
    pub fn modify(mut self, mut f: impl FnMut(&mut K)) -> Self {
        match &mut self {
            Event::Applied(obj) | Event::Deleted(obj) => (f)(obj),
            Event::Restarted(objs) => {
                for k in objs {
                    (f)(k)
                }
            }
        }
        self
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
/// The internal finite state machine driving the [`watcher`]
enum State<K: Resource + Clone> {
    /// The Watcher is empty, and the next [`poll`](Stream::poll_next) will start the initial LIST to get all existing objects
    Empty,
    /// The initial LIST was successful, so we should move on to starting the actual watch.
    InitListed { resource_version: String },
    /// The watch is in progress, from this point we just return events from the server.
    ///
    /// If the connection is disrupted then we propagate the error but try to restart the watch stream by
    /// returning to the `InitListed` state.
    /// If we fall out of the K8s watch window then we propagate the error and fall back doing a re-list
    /// with `Empty`.
    Watching {
        resource_version: String,
        #[derivative(Debug = "ignore")]
        stream: BoxStream<'static, kube_client::Result<WatchEvent<K>>>,
    },
}

/// Progresses the watcher a single step, returning (event, state)
///
/// This function should be trampolined: if event == `None`
/// then the function should be called again until it returns a Some.
async fn step_trampolined<K: Resource + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    state: State<K>,
) -> (Option<Result<Event<K>>>, State<K>) {
    match state {
        State::Empty => match api.list(list_params).await {
            Ok(list) => (Some(Ok(Event::Restarted(list.items))), State::InitListed {
                resource_version: list.metadata.resource_version.unwrap(),
            }),
            Err(err) => (Some(Err(err).map_err(Error::InitialListFailed)), State::Empty),
        },
        State::InitListed { resource_version } => match api.watch(list_params, &resource_version).await {
            Ok(stream) => (None, State::Watching {
                resource_version,
                stream: stream.boxed(),
            }),
            Err(err) => (
                Some(Err(err).map_err(Error::WatchStartFailed)),
                State::InitListed { resource_version },
            ),
        },
        State::Watching {
            resource_version,
            mut stream,
        } => match stream.next().await {
            Some(Ok(WatchEvent::Added(obj) | WatchEvent::Modified(obj))) => {
                let resource_version = obj.resource_version().unwrap();
                (Some(Ok(Event::Applied(obj))), State::Watching {
                    resource_version,
                    stream,
                })
            }
            Some(Ok(WatchEvent::Deleted(obj))) => {
                let resource_version = obj.resource_version().unwrap();
                (Some(Ok(Event::Deleted(obj))), State::Watching {
                    resource_version,
                    stream,
                })
            }
            Some(Ok(WatchEvent::Bookmark(bm))) => (None, State::Watching {
                resource_version: bm.metadata.resource_version,
                stream,
            }),
            Some(Ok(WatchEvent::Error(err))) => {
                // HTTP GONE, means we have desynced and need to start over and re-list :(
                let new_state = if err.code == 410 {
                    State::Empty
                } else {
                    State::Watching {
                        resource_version,
                        stream,
                    }
                };
                (Some(Err(err).map_err(Error::WatchError)), new_state)
            }
            Some(Err(err)) => (Some(Err(err).map_err(Error::WatchFailed)), State::Watching {
                resource_version,
                stream,
            }),
            None => (None, State::InitListed { resource_version }),
        },
    }
}

/// Trampoline helper for `step_trampolined`
async fn step<K: Resource + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    mut state: State<K>,
) -> (Result<Event<K>>, State<K>) {
    loop {
        match step_trampolined(api, list_params, state).await {
            (Some(result), new_state) => return (result, new_state),
            (None, new_state) => state = new_state,
        }
    }
}

/// Watches a Kubernetes Resource for changes continuously
///
/// Compared to [`Api::watch`], this automatically tries to recover the stream upon errors.
///
/// Errors from the underlying watch are propagated, after which the stream will go into recovery mode on the next poll.
/// You can apply your own backoff by not polling the stream for a duration after errors.
/// Keep in mind that some [`TryStream`](futures::TryStream) combinators (such as
/// [`try_for_each`](futures::TryStreamExt::try_for_each) and [`try_concat`](futures::TryStreamExt::try_concat))
/// will terminate eagerly as soon as they receive an [`Err`].
///
/// This is intended to provide a safe and atomic input interface for a state store like a [`reflector`].
/// Direct users may want to flatten composite events via [`WatchStreamExt`]:
///
/// ```no_run
/// use kube::{
///   api::{Api, ListParams, ResourceExt}, Client,
///   runtime::{watcher, WatchStreamExt}
/// };
/// use k8s_openapi::api::core::v1::Pod;
/// use futures::{StreamExt, TryStreamExt};
/// #[tokio::main]
/// async fn main() -> Result<(), watcher::Error> {
///     let client = Client::try_default().await.unwrap();
///     let pods: Api<Pod> = Api::namespaced(client, "apps");
///
///     watcher(pods, ListParams::default()).applied_objects()
///         .try_for_each(|p| async move {
///          println!("Applied: {}", p.name());
///             Ok(())
///         })
///         .await?;
///    Ok(())
/// }
/// ```
/// [`WatchStreamExt`]: super::WatchStreamExt
/// [`reflector`]: super::reflector::reflector
/// [`Api::watch`]: kube_client::Api::watch
///
/// # Recovery
///
/// The stream will attempt to be recovered on the next poll after an [`Err`] is returned.
/// This will normally happen immediately, but you can use [`StreamBackoff`](crate::utils::StreamBackoff)
/// to introduce an artificial delay. [`default_backoff`] returns a suitable default set of parameters.
///
/// If the watch connection is interrupted, then `watcher` will attempt to restart the watch using the last
/// [resource version](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
/// that we have seen on the stream. If this is successful then the stream is simply resumed from where it left off.
/// If this fails because the resource version is no longer valid then we start over with a new stream, starting with
/// an [`Event::Restarted`]. The internals mechanics of recovery should be considered an implementation detail.
pub fn watcher<K: Resource + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: Api<K>,
    list_params: ListParams,
) -> impl Stream<Item = Result<Event<K>>> + Send {
    futures::stream::unfold(
        (api, list_params, State::Empty),
        |(api, list_params, state)| async {
            let (event, state) = step(&api, &list_params, state).await;
            Some((event, (api, list_params, state)))
        },
    )
}

/// Watch a single named object for updates
///
/// Emits `None` if the object is deleted (or not found), and `Some` if an object is updated (or created/found).
///
/// Compared to [`watcher`], `watch_object` does not return return [`Event`], since there is no need for an atomic
/// [`Event::Restarted`] when only one object is covered anyway.
pub fn watch_object<K: Resource + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: Api<K>,
    name: &str,
) -> impl Stream<Item = Result<Option<K>>> + Send {
    watcher(api, ListParams {
        field_selector: Some(format!("metadata.name={name}")),
        ..Default::default()
    })
    .map(|event| match event? {
        Event::Deleted(_) => Ok(None),
        // We're filtering by object name, so getting more than one object means that either:
        // 1. The apiserver is accepting multiple objects with the same name, or
        // 2. The apiserver is ignoring our query
        // In either case, the K8s apiserver is broken and our API will return invalid data, so
        // we had better bail out ASAP.
        Event::Restarted(objs) if objs.len() > 1 => Err(Error::TooManyObjects),
        Event::Restarted(mut objs) => Ok(objs.pop()),
        Event::Applied(obj) => Ok(Some(obj)),
    })
}

/// Default watch [`Backoff`] inspired by Kubernetes' client-go.
///
/// Note that the exact parameters used herein should not be considered stable.
/// The parameters currently optimize for being kind to struggling apiservers.
/// See [client-go's reflector source](https://github.com/kubernetes/client-go/blob/980663e185ab6fc79163b1c2565034f6d58368db/tools/cache/reflector.go#L177-L181)
/// for more details.
#[must_use]
pub fn default_backoff() -> impl Backoff + Send + Sync {
    let expo = backoff::ExponentialBackoff {
        initial_interval: Duration::from_millis(800),
        max_interval: Duration::from_secs(30),
        randomization_factor: 1.0,
        multiplier: 2.0,
        max_elapsed_time: None,
        ..ExponentialBackoff::default()
    };
    ResetTimerBackoff::new(expo, Duration::from_secs(120))
}
