use crate::reflector::store::Writer;
use crate::reflector::Store;
use crate::watcher;
use kube_client::{Resource, ResourceExt};
use std::collections::HashSet;
use std::hash::Hash;

pub trait ToStore<K>
where
    K: 'static + Resource + Clone,
    K::DynamicType: Eq + Clone + Default + Hash,
{
    fn to_store(self) -> Store<K>;
}

impl<K> ToStore<K> for Vec<K>
where
    K: 'static + Resource + Clone,
    K::DynamicType: Eq + Clone + Default + Hash,
{
    fn to_store(self) -> Store<K> {
        check_for_duplicates(&self);

        let mut store_writer = Writer::default();
        store_writer.apply_watcher_event(&watcher::Event::Restarted(self));
        store_writer.as_reader()
    }
}

/// A Store converts the Vec into a HashMap with a key based on the resource's name and namespace
/// so they must be unique. We check for bad test data here to ensure we aren't trying to put
/// duplicate resources into the Store (which would cause any duplicates to disappear).
fn check_for_duplicates<K>(to_store: &[K])
where
    K: 'static + Resource + Clone,
    K::DynamicType: Default + Eq + Clone + Hash,
{
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for item in to_store {
        let key = (item.name_unchecked(), item.meta().namespace.clone());

        if seen.contains(&key) {
            duplicates.push(key);
        } else {
            seen.insert(key);
        }
    }

    assert!(
        duplicates.is_empty(),
        "Attempted .to_store() on a vec with with duplicate resources: {:?}",
        duplicates
    );
}
