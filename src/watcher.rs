use derivative::Derivative;
use futures::StreamExt;
use futures::{stream::LocalBoxStream, Stream};
use kube::{
    api::{ListParams, Meta, WatchEvent},
    Api,
};

use serde::de::DeserializeOwned;
use snafu::{Backtrace, ResultExt, Snafu};
use std::clone::Clone;

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

#[derive(Debug)]
/// Watch events returned from the `Watcher`
pub enum WatcherEvent<K> {
    /// A resource was added or modified
    Added(K),
    /// A resource was deleted
    ///
    /// NOTE: This should not be used for managing persistent state elsewhere, since
    /// events may be lost if the watcher is unavailable. Use Finalizers instead.
    Deleted(K),
    /// The watch stream was restarted, so `Deleted` events may have been missed
    ///
    /// Should be used as a signal to clear caches.
    Restarted,
}

#[derive(Derivative)]
#[derivative(Debug)]
/// The internal FSM driving the [`Watcher`](struct.Watcher.html)
///
/// NOTE: This isn't intended to be used externally or part of the external API,
/// but it's published to document the internal workings.
pub enum State<K: Meta + Clone> {
    /// The Watcher is empty, and the next poll() will start the initial LIST to get all existing objects
    Empty,
    /// The initial LIST was successful, so we return the existing objects as `Added` events one by one
    ///
    /// If the queue is empty then move on to starting the actual watch.
    InitListed {
        resource_version: String,
        queue: std::vec::IntoIter<K>,
    },
    /// The watch is in progress, from this point we just return events from the server.
    ///
    /// If the connection is disrupted then we propagate the error but try to restart the watch stream.
    /// If we fall out of the K8s watch window then we propagate the error and fall back doing a re-list
    /// with `Empty`.
    Watching {
        resource_version: String,
        #[derivative(Debug = "ignore")]
        stream: LocalBoxStream<'static, kube::Result<WatchEvent<K>>>,
    },
}

/// Progresses the watcher a single step, returning (event, state)
///
/// This function should be trampolined: if event == `None` then the function should be called
/// again until it returns a Some
async fn step_trampolined<K: Meta + Clone + DeserializeOwned + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    state: State<K>,
) -> (Option<Result<WatcherEvent<K>>>, State<K>) {
    match state {
        State::Empty => match api.list(&list_params).await {
            Ok(list) => (
                None,
                State::InitListed {
                    resource_version: list.metadata.resource_version.unwrap(),
                    queue: list.items.into_iter(),
                },
            ),
            Err(err) => (Some(Err(err).context(InitialListFailed)), State::Empty),
        },
        State::InitListed {
            resource_version,
            mut queue,
        } => match queue.next() {
            Some(obj) => (
                Some(Ok(WatcherEvent::Added(obj))),
                State::InitListed {
                    resource_version,
                    queue,
                },
            ),
            None => match api.watch(&list_params, &resource_version).await {
                Ok(stream) => (
                    None,
                    State::Watching {
                        resource_version,
                        stream: stream.boxed_local(),
                    },
                ),
                Err(err) => (
                    Some(Err(err).context(WatchStartFailed)),
                    State::InitListed {
                        resource_version,
                        queue,
                    },
                ),
            },
        },
        State::Watching {
            resource_version,
            mut stream,
        } => match stream.next().await {
            Some(Ok(WatchEvent::Added(obj))) | Some(Ok(WatchEvent::Modified(obj))) => {
                let resource_version = obj.resource_ver().unwrap();
                (
                    Some(Ok(WatcherEvent::Added(obj))),
                    State::Watching {
                        resource_version,
                        stream,
                    },
                )
            }
            Some(Ok(WatchEvent::Deleted(obj))) => {
                let resource_version = obj.resource_ver().unwrap();
                (
                    Some(Ok(WatcherEvent::Deleted(obj))),
                    State::Watching {
                        resource_version,
                        stream,
                    },
                )
            }
            Some(Ok(WatchEvent::Bookmark(obj))) => (
                None,
                State::Watching {
                    resource_version: obj.resource_ver().unwrap(),
                    stream,
                },
            ),
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
            Some(Err(err)) => (
                Some(Err(err).context(WatchFailed)),
                State::Watching {
                    resource_version,
                    stream,
                },
            ),
            None => (
                None,
                State::InitListed {
                    resource_version,
                    queue: Vec::new().into_iter(),
                },
            ),
        },
    }
}

/// Trampoline helper for `step_trampolined`
async fn step<K: Meta + Clone + DeserializeOwned + 'static>(
    api: &Api<K>,
    list_params: &ListParams,
    mut state: State<K>,
) -> (Result<WatcherEvent<K>>, State<K>) {
    loop {
        match step_trampolined(&api, &list_params, state).await {
            (Some(result), new_state) => return (result, new_state),
            (None, new_state) => state = new_state,
        }
    }
}

pub fn watcher<K: Meta + Clone + DeserializeOwned + 'static>(
    api: Api<K>,
    list_params: ListParams,
) -> impl Stream<Item = Result<WatcherEvent<K>>> {
    futures::stream::unfold(
        (api, list_params, State::Empty),
        |(api, list_params, state)| async {
            let (event, state) = step(&api, &list_params, state).await;
            Some((event, (api, list_params, state)))
        },
    )
}
