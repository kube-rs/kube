//! Caches objects in memory

mod object_ref;
mod store;

pub use self::object_ref::ObjectRef;
use crate::{utils, watcher};
use futures::{Stream, StreamExt, TryStreamExt};
use kube_client::{
    api::{Api, ListParams},
    Resource,
};
use serde::de::DeserializeOwned;
use std::{fmt::Debug, hash::Hash};
pub use store::{Store, Writer};

/// Caches objects from `watcher::Event`s to a local `Store`
///
/// Keep in mind that the `Store` is just a cache, and may be out of date.
///
/// Note: It is a bad idea to feed a single `reflector` from multiple `watcher`s, since
/// the whole `Store` will be cleared whenever any of them emits a `Restarted` event.
pub fn reflector<K, W>(mut store: store::Writer<K>, stream: W) -> impl Stream<Item = W::Item>
where
    K: Resource + Clone,
    K::DynamicType: Eq + Hash + Clone,
    W: Stream<Item = watcher::Result<watcher::Event<K>>>,
{
    stream.inspect_ok(move |event| store.apply_watcher_event(event))
}

/// A simple reflector cache around a store and an owned watcher
///
/// Requires `list` and `watch` access of the resource `K`.
pub struct Reflector<K>
where
    K: Clone + Resource + Send + Sync + 'static,
    K::DynamicType: Eq + Hash,
{
    // temporary builder params
    api: Api<K>,
    lp: ListParams,
    writer: Writer<K>,
}

impl<K> Reflector<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Eq + Hash + Clone + Default,
{
    /// Create a Reflector on a reflector on a type `K`
    ///
    /// Takes an [`Api`] object that determines how the `Reflector` listens for changes to the `K`.
    ///
    /// The [`ListParams`] controls to the possible subset of objects of `K` that you want to cache.
    /// For the full set of objects `K` in the given `Api` scope, you can use [`ListParams::default`].
    #[must_use]
    pub fn new(api: Api<K>, lp: ListParams) -> (Self, Store<K>) {
        Self::new_with(api, lp, Default::default())
    }
}

impl<K> Reflector<K>
where
    K: Clone + Resource + DeserializeOwned + Debug + Send + Sync + 'static,
    K::DynamicType: Eq + Hash + Clone,
{
    /// Create a Reflector on a reflector on a type `K`
    ///
    /// Takes an [`Api`] object that determines how the `Reflector` listens for changes to the `K`.
    ///
    /// The [`ListParams`] controls to the possible subset of objects of `K` that you want to cache.
    /// For the full set of objects `K` in the given `Api` scope, you can use [`ListParams::default`].
    ///
    /// This variant constructor is for [`dynamic`] types found through discovery. Prefer [`Reflector::new`] for static types.
    #[must_use]
    pub fn new_with(api: Api<K>, lp: ListParams, dyntype: K::DynamicType) -> (Self, Store<K>) {
        let writer = Writer::<K>::new(dyntype);
        let reader = writer.as_reader();
        (Self { api, lp, writer }, reader)
    }

    fn start(self) -> impl Stream<Item = watcher::Result<watcher::Event<K>>> {
        reflector(self.writer, watcher(self.api, self.lp))
    }

    /// Consume the stream and return a future that will run the duration of the program
    ///
    /// This should be awaited forever.
    ///
    /// # Errors
    ///
    /// If an irrecoverable [`watcher::Error`] was encountered, the first is returned:
    ///
    /// - 404 `ErrorResponse`(watching invalid / missing api kind/group for `K`)
    /// - 403 `ErrorResponse` (missing list + watch rbac verbs for `K`)
    pub async fn run(self) -> Result<(), watcher::Error> {
        let mut applies = utils::try_flatten_applied(self.start().boxed());
        while applies.try_next().await?.is_some() {}
        Ok(())
    }

    /// Consumes the cache, runs the reflector, and returns an information stream of watch events (modified/added)
    ///
    /// Note that the returned stream is always reflected in the [`reader`](Reflector::reader).
    /// If you do not require a reader, prefer using a [`watcher`] directly.
    pub fn watch_applies(self) -> impl Stream<Item = Result<K, watcher::Error>> {
        let stream = self.start();
        utils::try_flatten_applied(stream)
    }

    /// Consumes the cache, runs the reflector, and returns an informational stream of watch events (modified/added/deleted)
    ///
    /// Note that the returned stream is always reflected in the [`reader`](Reflector::reader).
    /// If you do not require a reader, prefer using a [`watcher`] directly.
    pub fn watch_touches(self) -> impl Stream<Item = Result<K, watcher::Error>> {
        let stream = self.start();
        utils::try_flatten_touched(stream)
    }
}


#[cfg(test)]
mod tests {
    use super::{reflector, store, ObjectRef};
    use crate::watcher;
    use futures::{stream, StreamExt, TryStreamExt};
    use k8s_openapi::{api::core::v1::ConfigMap, apimachinery::pkg::apis::meta::v1::ObjectMeta};
    use rand::{
        distributions::{Bernoulli, Uniform},
        Rng,
    };
    use std::collections::{BTreeMap, HashMap};

    #[tokio::test]
    async fn reflector_applied_should_add_object() {
        let store_w = store::Writer::default();
        let store = store_w.as_reader();
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("a".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        reflector(
            store_w,
            stream::iter(vec![Ok(watcher::Event::Applied(cm.clone()))]),
        )
        .map(|_| ())
        .collect::<()>()
        .await;
        assert_eq!(store.get(&ObjectRef::from_obj(&cm)), Some(cm));
    }

    #[tokio::test]
    async fn reflector_applied_should_update_object() {
        let store_w = store::Writer::default();
        let store = store_w.as_reader();
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("a".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let updated_cm = ConfigMap {
            data: Some({
                let mut data = BTreeMap::new();
                data.insert("data".to_string(), "present!".to_string());
                data
            }),
            ..cm.clone()
        };
        reflector(
            store_w,
            stream::iter(vec![
                Ok(watcher::Event::Applied(cm.clone())),
                Ok(watcher::Event::Applied(updated_cm.clone())),
            ]),
        )
        .map(|_| ())
        .collect::<()>()
        .await;
        assert_eq!(store.get(&ObjectRef::from_obj(&cm)), Some(updated_cm));
    }

    #[tokio::test]
    async fn reflector_deleted_should_remove_object() {
        let store_w = store::Writer::default();
        let store = store_w.as_reader();
        let cm = ConfigMap {
            metadata: ObjectMeta {
                name: Some("a".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        reflector(
            store_w,
            stream::iter(vec![
                Ok(watcher::Event::Applied(cm.clone())),
                Ok(watcher::Event::Deleted(cm.clone())),
            ]),
        )
        .map(|_| ())
        .collect::<()>()
        .await;
        assert_eq!(store.get(&ObjectRef::from_obj(&cm)), None);
    }

    #[tokio::test]
    async fn reflector_restarted_should_clear_objects() {
        let store_w = store::Writer::default();
        let store = store_w.as_reader();
        let cm_a = ConfigMap {
            metadata: ObjectMeta {
                name: Some("a".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        let cm_b = ConfigMap {
            metadata: ObjectMeta {
                name: Some("b".to_string()),
                ..ObjectMeta::default()
            },
            ..ConfigMap::default()
        };
        reflector(
            store_w,
            stream::iter(vec![
                Ok(watcher::Event::Applied(cm_a.clone())),
                Ok(watcher::Event::Restarted(vec![cm_b.clone()])),
            ]),
        )
        .map(|_| ())
        .collect::<()>()
        .await;
        assert_eq!(store.get(&ObjectRef::from_obj(&cm_a)), None);
        assert_eq!(store.get(&ObjectRef::from_obj(&cm_b)), Some(cm_b));
    }

    #[tokio::test]
    async fn reflector_store_should_not_contain_duplicates() {
        let mut rng = rand::thread_rng();
        let item_dist = Uniform::new(0_u8, 100);
        let deleted_dist = Bernoulli::new(0.40).unwrap();
        let store_w = store::Writer::default();
        let store = store_w.as_reader();
        reflector(
            store_w,
            stream::iter((0_u32..100_000).map(|gen| {
                let item = rng.sample(item_dist);
                let deleted = rng.sample(deleted_dist);
                let obj = ConfigMap {
                    metadata: ObjectMeta {
                        name: Some(item.to_string()),
                        resource_version: Some(gen.to_string()),
                        ..ObjectMeta::default()
                    },
                    ..ConfigMap::default()
                };
                Ok(if deleted {
                    watcher::Event::Deleted(obj)
                } else {
                    watcher::Event::Applied(obj)
                })
            })),
        )
        .map_ok(|_| ())
        .try_collect::<()>()
        .await
        .unwrap();

        let mut seen_objects = HashMap::new();
        for obj in store.state() {
            assert_eq!(seen_objects.get(obj.metadata.name.as_ref().unwrap()), None);
            seen_objects.insert(obj.metadata.name.clone().unwrap(), obj);
        }
    }
}
