use crate::watcher;
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

#[derive(Debug)]
pub struct Cache<K> {
    // DashMap isn't async-aware, but that's fine as long
    // as we never hold the lock over an async/await boundary
    store: DashMap<ObjectRef<K>, K>,
}

impl<K: Clone> Cache<K> {
    pub fn get(&self, key: &ObjectRef<K>) -> Option<K> {
        // Clone to let go of the entry lock ASAP
        self.store.get(key).map(|entry| entry.value().clone())
    }
}

/// Applies a single event to the cache
fn apply_to_cache<K: Meta + Clone>(cache: &Cache<K>, event: &watcher::Event<K>) {
    match event {
        watcher::Event::Added(obj) => {
            cache.store.insert(ObjectRef::from_obj(&obj), obj.clone());
        }
        watcher::Event::Deleted(obj) => {
            cache.store.remove(&ObjectRef::from_obj(&obj));
        }
        watcher::Event::Restarted(new_objs) => {
            let new_objs = new_objs
                .into_iter()
                .map(|obj| (ObjectRef::from_obj(obj), obj))
                .collect::<HashMap<_, _>>();
            // We can't do do the whole replacement atomically, but we should at least not delete objects that still exist
            cache
                .store
                .retain(|key, _old_value| new_objs.contains_key(key));
            for (key, obj) in new_objs {
                cache.store.insert(key, obj.clone());
            }
        }
    }
}

/// Caches objects locally
///
/// Similar to kube-rs's `Reflector`, and the caching half of client-go's `Reflector`
pub fn reflector<K: Meta + Clone, W: Stream<Item = watcher::Result<watcher::Event<K>>>>(
    cache: Cache<K>,
    stream: W,
) -> impl Stream<Item = W::Item> {
    stream.inspect_ok(move |event| apply_to_cache(&cache, event))
}
