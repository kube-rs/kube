use derivative::Derivative;
use futures::{future::BoxFuture, stream::LocalBoxStream, Stream};
use kube::{
    api::{ListParams, Meta, ObjectList, WatchEvent},
    Api,
};
use pin_project::pin_project;
use serde::de::DeserializeOwned;
use snafu::{Backtrace, ResultExt, Snafu};
use std::{
    clone::Clone,
    marker::{Send, Sync},
    pin::Pin,
    task::{Context, Poll},
};

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
type Result<T, E = Error> = std::result::Result<T, E>;

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
    /// The Watcher is performing the initial LIST to get all existing objects
    ///
    /// If this fails then we propagate the error and fall back to `Empty`, to retry on the next poll.
    InitListing {
        #[derivative(Debug = "ignore")]
        list_fut: BoxFuture<'static, kube::Result<ObjectList<K>>>,
    },
    /// The initial LIST was successful, so we return the existing objects as `Added` events one by one
    ///
    /// If the queue is empty then move on to starting the actual watch.
    InitListed {
        resource_version: String,
        queue: std::vec::IntoIter<K>,
    },
    /// We're initializing the watch stream, hold tight!
    ///
    /// If this fails then we move back to an empty `InitListed`, and retry on the next poll.
    InitWatching {
        resource_version: String,
        #[derivative(Debug = "ignore")]
        stream_fut:
            BoxFuture<'static, kube::Result<LocalBoxStream<'static, kube::Result<WatchEvent<K>>>>>,
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

#[pin_project]
#[derive(Derivative)]
#[derivative(Debug)]
/// Watches a Kubernetes Resource for changes
///
/// Errors are propagated to the client, but can continue to be polled, in which case it tries to recover
/// from the error.
pub struct Watcher<K: Meta + Clone> {
    #[derivative(Debug = "ignore")]
    api: Api<K>,
    #[derivative(Debug = "ignore")]
    list_params: ListParams,
    state: State<K>,
}

impl<K: Meta + Clone + DeserializeOwned + 'static> Watcher<K> {
    pub fn new(api: Api<K>, list_params: ListParams) -> Self {
        Self {
            api,
            list_params,
            state: State::Empty,
        }
    }
}

async fn list_owning_wrapper<K: Meta + Clone + DeserializeOwned>(
    api: Api<K>,
    lp: ListParams,
) -> kube::Result<ObjectList<K>> {
    api.list(&lp).await
}

async fn watch_owning_wrapper<K: Meta + Clone + DeserializeOwned + 'static>(
    api: Api<K>,
    lp: ListParams,
    version: String,
) -> kube::Result<Pin<Box<dyn Stream<Item = kube::Result<WatchEvent<K>>>>>> {
    api.watch(&lp, &version)
        .await
        .map(|x| Box::pin(x) as Pin<Box<dyn Stream<Item = _>>>)
}

impl<K: Sync + Send + Meta + Clone + DeserializeOwned + 'static> Stream for Watcher<K> {
    type Item = Result<WatcherEvent<K>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.as_mut().project();
        match this.state {
            State::Empty => {
                *this.state = State::InitListing {
                    list_fut: Box::pin(list_owning_wrapper(
                        this.api.clone(),
                        this.list_params.clone(),
                    )),
                };
                self.poll_next(cx)
            }
            State::InitListing { list_fut } => match list_fut.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(list)) => {
                    *this.state = State::InitListed {
                        resource_version: list.metadata.resource_version.unwrap(),
                        queue: list.items.into_iter(),
                    };
                    self.poll_next(cx)
                }
                Poll::Ready(Err(err)) => {
                    // Reset the internal state, so we retry on the next poll
                    *this.state = State::Empty;
                    Poll::Ready(Some(Err(err).context(InitialListFailed)))
                }
            },
            State::InitListed {
                resource_version,
                queue,
            } => match queue.next() {
                Some(obj) => Poll::Ready(Some(Ok(WatcherEvent::Added(obj)))),
                None => {
                    *this.state = State::InitWatching {
                        resource_version: resource_version.clone(),
                        stream_fut: Box::pin(watch_owning_wrapper(
                            this.api.clone(),
                            this.list_params.clone(),
                            resource_version.clone(),
                        )),
                    };
                    self.poll_next(cx)
                }
            },
            State::InitWatching {
                resource_version,
                stream_fut,
            } => match stream_fut.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(stream)) => {
                    *this.state = State::Watching {
                        resource_version: resource_version.clone(),
                        stream,
                    };
                    self.poll_next(cx)
                }
                // FIXME: Handle invalid resource version separately
                Poll::Ready(Err(err)) => {
                    *this.state = State::InitListed {
                        resource_version: resource_version.clone(),
                        queue: Vec::new().into_iter(),
                    };
                    Poll::Ready(Some(Err(err).context(WatchStartFailed)))
                }
            },
            State::Watching {
                resource_version,
                stream,
            } => match stream.as_mut().poll_next(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Some(Ok(WatchEvent::Added(obj)))) => {
                    *resource_version = obj.resource_ver().unwrap();
                    Poll::Ready(Some(Ok(WatcherEvent::Added(obj))))
                }
                Poll::Ready(Some(Ok(WatchEvent::Modified(obj)))) => {
                    *resource_version = obj.resource_ver().unwrap();
                    Poll::Ready(Some(Ok(WatcherEvent::Added(obj))))
                }
                Poll::Ready(Some(Ok(WatchEvent::Deleted(obj)))) => {
                    *resource_version = obj.resource_ver().unwrap();
                    Poll::Ready(Some(Ok(WatcherEvent::Deleted(obj))))
                }
                Poll::Ready(Some(Ok(WatchEvent::Bookmark(obj)))) => {
                    *resource_version = obj.resource_ver().unwrap();
                    self.poll_next(cx)
                }
                Poll::Ready(Some(Ok(WatchEvent::Error(err)))) => {
                    Poll::Ready(Some(Err(err).context(WatchError)))
                }
                Poll::Ready(Some(Err(err))) => Poll::Ready(Some(Err(err).context(WatchFailed))),
                Poll::Ready(None) => {
                    *this.state = State::InitListed {
                        resource_version: resource_version.clone(),
                        queue: Vec::new().into_iter(),
                    };
                    self.poll_next(cx)
                }
            },
        }
    }
}
