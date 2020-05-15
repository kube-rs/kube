mod object_ref;
mod store;

pub use self::object_ref::{ErasedResource, ObjectRef};
use crate::watcher;
use futures::{Stream, TryStreamExt};
use kube::api::Meta;
use std::collections::HashMap;
pub use store::Store;

/// Applies a single event to the store
fn apply_to_store<K: Meta + Clone>(store: &Store<K>, event: &watcher::Event<K>) {
    match event {
        watcher::Event::Added(obj) => {
            store.store.insert(ObjectRef::from_obj(&obj), obj.clone());
        }
        watcher::Event::Deleted(obj) => {
            store.store.remove(&ObjectRef::from_obj(&obj));
        }
        watcher::Event::Restarted(new_objs) => {
            let new_objs = new_objs
                .iter()
                .map(|obj| (ObjectRef::from_obj(obj), obj))
                .collect::<HashMap<_, _>>();
            // We can't do do the whole replacement atomically, but we should at least not delete objects that still exist
            store
                .store
                .retain(|key, _old_value| new_objs.contains_key(key));
            for (key, obj) in new_objs {
                store.store.insert(key, obj.clone());
            }
        }
    }
}

/// Caches objects to a local store
///
/// Similar to kube-rs's `Reflector`, and the caching half of client-go's `Reflector`
pub fn reflector<K: Meta + Clone, W: Stream<Item = watcher::Result<watcher::Event<K>>>>(
    store: Store<K>,
    stream: W,
) -> impl Stream<Item = W::Item> {
    stream.inspect_ok(move |event| apply_to_store(&store, event))
}
