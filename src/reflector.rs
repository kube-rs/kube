use crate::watcher::{self, WatcherEvent};
use dashmap::DashMap;
use derivative::Derivative;
use futures::Stream;
use kube::api::Meta;
use pin_project::pin_project;
use std::{marker::PhantomData, pin::Pin, task::Poll};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ObjectRef<K> {
    _type: PhantomData<K>,
    pub name: String,
    pub namespace: Option<String>,
}

impl<K: Meta> ObjectRef<K> {
    pub fn new_namespaced(name: String, namespace: String) -> Self {
        Self {
            _type: PhantomData,
            name,
            namespace: Some(namespace),
        }
    }

    pub fn new_clusterscoped(name: String) -> Self {
        Self {
            _type: PhantomData,
            name,
            namespace: None,
        }
    }

    pub fn from_obj(obj: &K) -> Self {
        Self {
            _type: PhantomData,
            name: obj.name(),
            namespace: obj.namespace(),
        }
    }
}

pub type Cache<K> = DashMap<ObjectRef<K>, K>;

/// Caches objects locally
///
/// Similar to kube-rs's `Reflector`, and the caching half of client-go's `Reflector`
#[pin_project]
pub struct Reflector<K, W> {
    #[pin]
    watcher: W,
    cache: Cache<K>,
}

impl<K, W> Reflector<K, W> {
    pub fn new(cache: Cache<K>, watcher: W) -> Self {
        Self { cache, watcher }
    }
}

impl<K: Meta + Clone, W: Stream<Item = watcher::Result<WatcherEvent<K>>>> Stream
    for Reflector<K, W>
{
    type Item = watcher::Result<WatcherEvent<K>>;
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let poll = this.watcher.poll_next(cx);
        match &poll {
            // Nested match to get exhaustiveness checking for the branches that we care about
            Poll::Ready(Some(Ok(event))) => match event {
                WatcherEvent::Added(obj) => {
                    this.cache.insert(ObjectRef::from_obj(&obj), obj.clone());
                }
                WatcherEvent::Deleted(obj) => {
                    this.cache.remove(&ObjectRef::from_obj(&obj));
                }
                WatcherEvent::Restarted => {
                    this.cache.clear();
                }
            },
            _ => {}
        }
        poll
    }
}
