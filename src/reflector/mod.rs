mod object_ref;
mod store;

pub use self::object_ref::{ErasedResource, ObjectRef};
use crate::watcher;
use futures::{Stream, TryStreamExt};
use kube::api::Meta;
pub use store::{Store, StoreWriter};

/// Caches objects to a local store
///
/// Similar to kube-rs's `Reflector`, and the caching half of client-go's `Reflector`
pub fn reflector<K: Meta + Clone, W: Stream<Item = watcher::Result<watcher::Event<K>>>>(
    mut store: StoreWriter<K>,
    stream: W,
) -> impl Stream<Item = W::Item> {
    stream.inspect_ok(move |event| store.apply_watcher_event(event))
}
