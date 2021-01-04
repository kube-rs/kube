use super::ObjectRef;
use crate::watcher;
use dashmap::DashMap;
use derivative::Derivative;
use k8s_openapi::Resource;
use kube::api::Meta;
use std::{collections::HashMap, fmt::Debug, sync::Arc};

/// A writable Store handle
///
/// This is exclusive since it's not safe to share a single `Store` between multiple reflectors.
/// In particular, `Restarted` events will clobber the state of other connected reflectors.
#[derive(Debug, Derivative)]
#[derivative(Default(bound = ""))]
pub struct Writer<K: 'static + Resource> {
    store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: 'static + Meta + Clone> Writer<K> {
    /// Return a read handle to the store
    ///
    /// Multiple read handles may be obtained, by either calling `as_reader` multiple times,
    /// or by calling `Store::clone()` afterwards.
    #[must_use]
    pub fn as_reader(&self) -> Store<K> {
        Store {
            store: self.store.clone(),
        }
    }

    /// Applies a single watcher event to the store
    pub fn apply_watcher_event(&mut self, event: &watcher::Event<K>) {
        match event {
            watcher::Event::Applied(obj) => {
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
                self.store.retain(|key, _old_value| new_objs.contains_key(key));
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
/// use `Writer::as_reader()` instead.
#[derive(Debug, Derivative)]
#[derivative(Clone)]
pub struct Store<K: 'static + Resource> {
    store: Arc<DashMap<ObjectRef<K>, K>>,
}

impl<K: 'static + Clone + Resource> Store<K> {
    /// Retrieve a `clone()` of the entry referred to by `key`, if it is in the cache.
    ///
    /// `key.namespace` is ignored for cluster-scoped resources.
    ///
    /// Note that this is a cache and may be stale. Deleted objects may still exist in the cache
    /// despite having been deleted in the cluster, and new objects may not yet exist in the cache.
    /// If any of these are a problem for you then you should abort your reconciler and retry later.
    /// If you use `kube_rt::controller` then you can do this by returning an error and specifying a
    /// reasonable `error_policy`.
    #[must_use]
    pub fn get(&self, key: &ObjectRef<K>) -> Option<K> {
        self.store
            .get(key)
            // Try to erase the namespace and try again, in case the object is cluster-scoped
            .or_else(|| {
                self.store.get(&{
                    let mut cluster_key = key.clone();
                    cluster_key.namespace = None;
                    cluster_key
                })
            })
            // Clone to let go of the entry lock ASAP
            .map(|entry| entry.value().clone())
    }

    /// Return a full snapshot of the current values
    #[must_use]
    pub fn state(&self) -> Vec<K> {
        self.store.iter().map(|eg| eg.value().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::Writer;
    use crate::{reflector::ObjectRef, watcher};
    use k8s_openapi::api::core::v1::ConfigMap;
    use kube::api::ObjectMeta;

    #[test]
    fn should_allow_getting_namespaced_object_by_namespaced_ref() {
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("obj".to_string()),
                namespace: Some("ns".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let mut store_w = Writer::default();
        store_w.apply_watcher_event(&watcher::Event::Applied(cm.clone()));
        let store = store_w.as_reader();
        assert_eq!(store.get(&ObjectRef::from_obj(&cm)), Some(cm));
    }

    #[test]
    fn should_not_allow_getting_namespaced_object_by_clusterscoped_ref() {
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("obj".to_string()),
                namespace: Some("ns".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let mut cluster_cm = cm.clone();
        cluster_cm.metadata.namespace = None;
        let mut store_w = Writer::default();
        store_w.apply_watcher_event(&watcher::Event::Applied(cm));
        let store = store_w.as_reader();
        assert_eq!(store.get(&ObjectRef::from_obj(&cluster_cm)), None);
    }

    #[test]
    fn should_allow_getting_clusterscoped_object_by_clusterscoped_ref() {
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("obj".to_string()),
                namespace: None,
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let mut store_w = Writer::default();
        store_w.apply_watcher_event(&watcher::Event::Applied(cm.clone()));
        let store = store_w.as_reader();
        assert_eq!(store.get(&ObjectRef::from_obj(&cm)), Some(cm));
    }

    #[test]
    fn should_allow_getting_clusterscoped_object_by_namespaced_ref() {
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("obj".to_string()),
                namespace: None,
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let mut nsed_cm = cm.clone();
        nsed_cm.metadata.namespace = Some("ns".to_string());
        let mut store_w = Writer::default();
        store_w.apply_watcher_event(&watcher::Event::Applied(cm.clone()));
        let store = store_w.as_reader();
        assert_eq!(store.get(&ObjectRef::from_obj(&nsed_cm)), Some(cm));
    }
}
