use crate::reflector::ObjectRef;
use std::{hash::Hash, collections::HashMap};
use kube_client::Resource;

/// A user owned evalation cache
///
/// This can be stored on the reconciler's context to check if reconciliation
/// is necessary checking it against a corresponding `Predicate`.
/// TODO: can we somehow bundle this with the predicate?
/// TODO: should we lock down V to be a hash of the actual value?
pub type Evaluations<K, V> = HashMap<ObjectRef<K>, V>;

/// A trait for predicate functions for controllers
///
/// Given a predicate fn from [`predicates`], we can call `cmp_update` with a
/// mutable evaluation store and an object, and we will see if the evaluation
/// gives a new values.
pub trait Predicate<K: Resource, V> {
    // Evaluate an object and check that it matches the
    fn cmp_update(&self, store: &mut Evaluations<K, V>, obj: &K) -> bool;
}

// This implements Predicate for all functions in the predicates module
impl<K: Resource, V: PartialEq, F: (Fn(&K) -> Option<V>)> Predicate<K, V> for F
where
    K::DynamicType: Default + Eq + Hash,
{
    fn cmp_update(&self, cache: &mut Evaluations<K, V>, obj: &K) -> bool {
        if let Some(val) = (self)(obj) {
            let key = ObjectRef::from_obj(obj);
            let changed = if let Some(old) = cache.get(&key) {
                *old != val // changed if key different
            } else {
                true // always changed if not in map
            };
            if let Some(old) = cache.get_mut(&key) {
                *old = val;
            } else {
                cache.insert(key, val);
            }
            changed
        } else {
            true
        }
    }
}

pub mod predicates {
    use kube_client::Resource;
    use std::collections::BTreeMap;
    // TODO: import from https://github.com/kubernetes-sigs/controller-runtime/blob/v0.12.0/pkg/predicate/predicate.go

    /// Compute the generation of a Resource K
    pub fn generation<K: Resource>(x: &K) -> Option<i64> {
        x.meta().generation
    }

    /// TODO: does this even impl Predicate? needs PartialEq on return type...
    /// TODO: hash these? users don't actually need a per-object label cache...
    pub fn labels<K: Resource>(x: &K) -> Option<BTreeMap<String, String>> {
        x.meta().labels.clone()
    }
}
