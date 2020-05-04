use crate::watcher::{self, WatcherEvent};
use dashmap::DashMap;
use derivative::Derivative;
use futures::{Stream, TryStreamExt};
use kube::api::Meta;
use std::{collections::HashMap, marker::PhantomData};

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
        WatcherEvent::Restarted(new_objs) => {
            let new_objs = new_objs
                .into_iter()
                .map(|obj| (ObjectRef::from_obj(obj), obj))
                .collect::<HashMap<_, _>>();
            // We can't do do the whole replacement atomically, but we should at least not delete objects that still exist
            cache.retain(|key, _old_value| new_objs.contains_key(key));
            for (key, obj) in new_objs.into_iter() {
                cache.insert(key, obj.clone());
            }
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
