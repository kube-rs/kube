use crate::watcher::{self, WatcherEvent};
use dashmap::DashMap;
use derivative::Derivative;
use futures::{Stream, TryStreamExt};
use kube::api::Meta;
use std::marker::PhantomData;

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

/// Applies a single event to the cache
fn apply_to_cache<K: Meta + Clone>(cache: &Cache<K>, event: &WatcherEvent<K>) {
    match event {
        WatcherEvent::Added(obj) => {
            cache.insert(ObjectRef::from_obj(&obj), obj.clone());
        }
        WatcherEvent::Deleted(obj) => {
            cache.remove(&ObjectRef::from_obj(&obj));
        }
        WatcherEvent::Restarted => {
            cache.clear();
        }
    }
}

/// Caches objects locally
///
/// Similar to kube-rs's `Reflector`, and the caching half of client-go's `Reflector`
pub fn reflector<K: Meta + Clone, W: Stream<Item = watcher::Result<WatcherEvent<K>>>>(
    cache: Cache<K>,
    stream: W,
) -> impl Stream<Item = W::Item> {
    stream.inspect_ok(move |event| apply_to_cache(&cache, event))
}
