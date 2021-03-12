//! Watches a Kubernetes Resource for changes, with error recovery

use derivative::Derivative;
use futures::{stream::BoxStream, Stream, StreamExt};
use kube::{
    api::{ListParams, Meta, WatchEvent},
    Api,
};
use serde::de::DeserializeOwned;
use smallvec::SmallVec;
use snafu::{Backtrace, ResultExt, Snafu};
use std::{clone::Clone, fmt::Debug};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("failed to perform initial object list: {}", source))]
    InitialListFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("failed to start watching object: {}", source))]
    WatchStartFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("error returned by apiserver during watch: {}", source))]
    WatchError {
        source: kube::error::ErrorResponse,
        backtrace: Backtrace,
    },
    #[snafu(display("watch stream failed: {}", source))]
    WatchFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
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
}

#[derive(Derivative)]
#[derivative(Debug)]
/// The internal finite state machine driving the [`watcher`]
enum State<K: Meta + Clone> {
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
        stream: BoxStream<'static, kube::Result<WatchEvent<K>>>,
    },
}

/// Progresses the watcher a single step, returning (event, state)
///
/// This function should be trampolined: if event == `None`
/// then the function should be called again until it returns a Some.
async fn step_trampolined<K: Meta + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    state: State<K>,
) -> (Option<Result<Event<K>>>, State<K>) {
    match state {
        State::Empty => match api.list(&list_params).await {
            Ok(list) => (Some(Ok(Event::Restarted(list.items))), State::InitListed {
                resource_version: list.metadata.resource_version.unwrap(),
            }),
            Err(err) => (Some(Err(err).context(InitialListFailed)), State::Empty),
        },
        State::InitListed { resource_version } => match api.watch(&list_params, &resource_version).await {
            Ok(stream) => (None, State::Watching {
                resource_version,
                stream: stream.boxed(),
            }),
            Err(err) => (Some(Err(err).context(WatchStartFailed)), State::InitListed {
                resource_version,
            }),
        },
        State::Watching {
            resource_version,
            mut stream,
        } => match stream.next().await {
            Some(Ok(WatchEvent::Added(obj))) | Some(Ok(WatchEvent::Modified(obj))) => {
                let resource_version = obj.resource_ver().unwrap();
                (Some(Ok(Event::Applied(obj))), State::Watching {
                    resource_version,
                    stream,
                })
            }
            Some(Ok(WatchEvent::Deleted(obj))) => {
                let resource_version = obj.resource_ver().unwrap();
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
                (Some(Err(err).context(WatchError)), new_state)
            }
            Some(Err(err)) => (Some(Err(err).context(WatchFailed)), State::Watching {
                resource_version,
                stream,
            }),
            None => (None, State::InitListed { resource_version }),
        },
    }
}

/// Trampoline helper for `step_trampolined`
async fn step<K: Meta + Clone + DeserializeOwned + Debug + Send + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    mut state: State<K>,
) -> (Result<Event<K>>, State<K>) {
    loop {
        match step_trampolined(&api, &list_params, state).await {
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
/// This is intended to provide a safe and atomic input interface for a state store like a [`reflector`],
/// direct users may want to flatten composite events with [`try_flatten_applied`]:
///
/// ```no_run
/// use kube::{api::{Api, ListParams, Meta}, Client};
/// use kube_runtime::{utils::try_flatten_applied, watcher};
/// use k8s_openapi::api::core::v1::Pod;
/// use futures::{StreamExt, TryStreamExt};
/// #[tokio::main]
/// async fn main() -> Result<(), kube_runtime::watcher::Error> {
///     let client = Client::try_default().await.unwrap();
///     let pods: Api<Pod> = Api::namespaced(client, "apps");
///     let watcher = watcher(pods, ListParams::default());
///     try_flatten_applied(watcher)
///         .try_for_each(|p| async move {
///          println!("Applied: {}", Meta::name(&p));
///             Ok(())
///         })
///         .await?;
///    Ok(())
/// }
/// ```
/// [`try_flatten_applied`]: super::utils::try_flatten_applied
/// [`reflector`]: super::reflector::reflector
/// [`Api::watch`]: https://docs.rs/kube/*/kube/struct.Api.html#method.watch
///
/// # Migration from `kube::runtime`
///
/// This is similar to the legacy [`kube::runtime::Informer`], or the watching half of client-go's `Reflector`.
/// Renamed to avoid confusion with client-go's `Informer` (which watches a `Reflector` for updates, rather
/// the Kubernetes API).
///
/// # Recovery
///
/// (The details of recovery are considered an implementation detail and should not be relied on to be stable, but are
/// documented here for posterity.)
///
/// If the watch connection is interrupted then we attempt to restart the watch using the last
/// [resource versions](https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes)
/// that we have seen on the stream. If this is successful then the stream is simply resumed from where it left off.
/// If this fails because the resource version is no longer valid then we start over with a new stream, starting with
/// an [`Event::Restarted`].
pub fn watcher<K: Meta + Clone + DeserializeOwned + Debug + Send + 'static>(
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
