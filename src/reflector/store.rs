use super::ObjectRef;
use crate::watcher;
// DashMap isn't async-aware, but that's fine as long
// as we never hold the lock over an async/await boundary
use dashmap::DashMap;
use derivative::Derivative;
use k8s_openapi::Resource;
use kube::api::Meta;
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

/// A writable Store handle
///
/// This is exclusive since it's not safe to share a single `Store` between multiple reflectors.
/// In particular, `Restarted` events will clobber the state of other connected reflectors.
#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct StoreWriter<K: Resource> {
    store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: Meta + Clone> StoreWriter<K> {
    /// Return a read handle to the store
    ///
    /// Multiple read handles may be obtained, by either calling `as_reader` multiple times,
    /// or by calling `Store::clone()` afterwards.
    pub fn as_reader(&self) -> Store<K> {
        Store {
            store: self.store.clone(),
        }
    }

    /// Applies a single watcher event to the store
    pub fn apply_watcher_event(&mut self, event: &watcher::Event<K>) {
        match event {
            watcher::Event::Added(obj) => {
                self.store.insert(ObjectRef::from_obj(&obj), obj.clone());
            }
            watcher::Event::Deleted(obj) => {
                self.store.remove(&ObjectRef::from_obj(&obj));
            }
            watcher::Event::Restarted(new_objs) => {
                let new_objs = new_objs
                    .iter()
                    .map(|obj| (ObjectRef::from_obj(obj), obj))
                    .collect::<HashMap<_, _>>();
                // We can't do do the whole replacement atomically, but we should at least not delete objects that still exist
                self.store
                    .retain(|key, _old_value| new_objs.contains_key(key));
                for (key, obj) in new_objs {
                    self.store.insert(key, obj.clone());
                }
            }
        }
    }
}

/// A readable cache of Kubernetes objects of kind `K`
///
/// Cloning will produce a new reference to the same backing store.
///
/// Cannot be constructed directly since one writer handle is required,
/// use `StoreWriter::as_reader()` instead.
#[derive(Debug, Derivative)]
#[derivative(Clone)]
pub struct Store<K: Resource> {
    // DashMap isn't async-aware, but that's fine as long
    // as we never hold the lock over an async/await boundary
    store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: Clone + Resource> Store<K> {
    /// Retrieve a `clone()` of the entry referred to by `key`, if it is in the cache.
    ///
    /// Note that this is a cache and may be stale. Deleted objects may still exist in the cache
    /// despite having been deleted in the cluster, and new objects may not yet exist in the cache.
    /// If any of these are a problem for you then you should abort your reconciler and retry later.
    /// If you use `kube_rt::controller` then you can do this by returning an error and specifying a
    /// reasonable `error_policy`.
    #[must_use]
    pub fn get(&self, key: &ObjectRef<K>) -> Option<K> {
        // Clone to let go of the entry lock ASAP
        self.store.get(key).map(|entry| entry.value().clone())
    }
}
