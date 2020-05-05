use crate::watcher;
use dashmap::DashMap;
use derivative::Derivative;
use futures::{Stream, TryStreamExt};
use k8s_openapi::Resource;
use kube::api::Meta;
use std::{collections::HashMap, sync::Arc};
use std::{hash::Hash, fmt::Debug};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ObjectRef<K: RuntimeResource> {
    kind: K::State,
    pub name: String,
    pub namespace: Option<String>,
}

impl<K: Meta> ObjectRef<K> {
    pub fn new_namespaced(name: String, namespace: String) -> Self {
        Self {
            kind: (),
            name,
            namespace: Some(namespace),
        }
    }

    pub fn new_clusterscoped(name: String) -> Self {
        Self {
            kind: (),
            name,
            namespace: None,
        }
    }

    pub fn from_obj(obj: &K) -> Self {
        Self {
            kind: (),
            name: obj.name(),
            namespace: obj.namespace(),
        }
    }
}

pub trait RuntimeResource {
    type State: Debug + PartialEq + Eq + Hash + Clone;
    fn group(state: &Self::State) -> &str;
    fn version(state: &Self::State) -> &str;
    fn kind(state: &Self::State) -> &str;
}

impl<K: Resource> RuntimeResource for K {
    // All required state is provided at build time
    type State = ();
    fn group(_state: &Self::State) -> &str {
        K::GROUP
    }
    fn version(_state: &Self::State) -> &str {
        K::VERSION
    }
    fn kind(_state: &Self::State) -> &str {
        K::KIND
    }
}

pub enum ErasedResource {}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ErasedResourceState {
    group: &'static str,
    version: &'static str,
    kind: &'static str,
}
impl RuntimeResource for ErasedResource {
    type State = ErasedResourceState;
    fn group(state: &Self::State) -> &str {
        &state.group
    }
    fn version(state: &Self::State) -> &str {
        &state.version
    }
    fn kind(state: &Self::State) -> &str {
        &state.kind
    }
}

impl ErasedResource {
    fn erase<K: Resource>() -> ErasedResourceState {
        ErasedResourceState {
            group: K::GROUP,
            version: K::VERSION,
            kind: K::KIND,
        }
    }
}

impl<K: Resource> From<ObjectRef<K>> for ObjectRef<ErasedResource> {
    fn from(old: ObjectRef<K>) -> Self {
        ObjectRef {
            kind: ErasedResource::erase::<K>(),
            name: old.name,
            namespace: old.namespace,
        }
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default, Clone)]
pub struct Cache<K: Resource> {
    // DashMap isn't async-aware, but that's fine as long
    // as we never hold the lock over an async/await boundary
    store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: Clone + Resource> Cache<K> {
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
