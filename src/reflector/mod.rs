mod object_ref;
mod store;

pub use self::object_ref::{ErasedResource, ObjectRef};
use crate::watcher;
use futures::{Stream, TryStreamExt};
use kube::api::Meta;
use std::collections::HashMap;
pub use store::Cache;

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
                .iter()
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
