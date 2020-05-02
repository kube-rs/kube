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
    InitialListFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
    WatchStartFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
    WatchError {
        source: kube::error::ErrorResponse,
        backtrace: Backtrace,
    },
    WatchFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
}
type Result<T, E = Error> = std::result::Result<T, E>;

pub enum WatcherEvent<K> {
    Added(K),
    Deleted(K),
}

#[derive(Derivative)]
#[derivative(Debug)]
enum State<K: Meta + Clone> {
    Empty,
    InitListing {
        #[derivative(Debug = "ignore")]
        list_fut: BoxFuture<'static, kube::Result<ObjectList<K>>>,
    },
    InitListed {
        resource_version: String,
        queue: std::vec::IntoIter<K>,
    },
    InitWatching {
        resource_version: String,
        #[derivative(Debug = "ignore")]
        stream_fut:
            BoxFuture<'static, kube::Result<LocalBoxStream<'static, kube::Result<WatchEvent<K>>>>>,
    },
    Watching {
        resource_version: String,
        #[derivative(Debug = "ignore")]
        stream: LocalBoxStream<'static, kube::Result<WatchEvent<K>>>,
    },
}

#[pin_project]
#[derive(Derivative)]
#[derivative(Debug)]
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
