use crate::reflector::{ObjectRef, Store};
use crate::shared_store::ready_token::ReadyToken;
use kube_client::Resource;
use std::hash::Hash;
use std::sync::Arc;

/// A wrapper around a Store that exposes async versions of the Store's methods. These will always
/// return immediately if the store is ready, or wait until the store is ready if it is not.
#[derive(Clone)]
pub struct SafeStore<K: 'static + Resource>
where
    K::DynamicType: Hash + Eq,
{
    store: Store<K>,
    ready: ReadyToken,
}

impl<K: 'static + Clone + Resource> SafeStore<K>
where
    K::DynamicType: Eq + Hash + Clone,
{
    pub fn new(store: Store<K>) -> SafeStore<K> {
        Self {
            store,
            ready: ReadyToken::new(),
        }
    }

    pub fn make_ready(&self) {
        self.ready.make_ready()
    }

    pub async fn get(&self, key: &ObjectRef<K>) -> Option<Arc<K>> {
        self.ready.ready().await;
        self.store.get(key)
    }

    pub async fn state(&self) -> Vec<Arc<K>> {
        self.ready.ready().await;
        self.store.state()
    }

    pub fn store(&self) -> Store<K> {
        self.store.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::shared_store::Writer;
    use crate::watcher::Event;
    use futures::FutureExt;
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn it_does_not_return_get_until_ready() {
        let store = test_store();
        let ss = SafeStore::<Pod>::new(store.clone());
        let obj_ref = &ObjectRef::new("test");

        let mut fut = ss.get(obj_ref).boxed();
        assert!((&mut fut).now_or_never().is_none());

        ss.make_ready();
        assert_eq!(
            (&mut fut).now_or_never().expect("Should have resolved"),
            store.get(obj_ref)
        );
    }

    #[test]
    fn it_does_not_return_state_until_ready() {
        let store = test_store();
        let ss = SafeStore::<Pod>::new(store.clone());

        let mut fut = ss.state().boxed();
        assert!((&mut fut).now_or_never().is_none());

        ss.make_ready();
        assert_eq!(
            (&mut fut).now_or_never().expect("Should have resolved"),
            store.state()
        );
    }

    fn test_store() -> Store<Pod> {
        let mut store_writer = Writer::default();
        store_writer.apply_watcher_event(&Event::Restarted(vec![{
            let mut pod = Pod::default();
            pod.meta_mut().name = Some("test".to_string());
            pod
        }]));
        store_writer.as_reader()
    }
}
