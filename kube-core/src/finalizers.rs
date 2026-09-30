//! Generic helpers for managing finalizers on resources.
//!
//! These helpers only mutate the object in memory; persisting the change (e.g. via a patch) is
//! up to the caller. For reconcilers, prefer `kube_runtime::finalizer`
//! (<https://docs.rs/kube/latest/kube/runtime/finalizer/fn.finalizer.html>): it persists the
//! finalizer to the apiserver *before* any work starts, which adding it in memory and writing it
//! back later does not guarantee.
use crate::resource::{Resource, ResourceExt};

/// Returns whether `obj` has the given finalizer
///
/// ```
/// use k8s_openapi::api::core::v1::Pod;
/// use kube_core::{add_finalizer, has_finalizer};
///
/// let mut pod = Pod::default();
/// assert!(!has_finalizer(&pod, "my.finalizer"));
/// add_finalizer(&mut pod, "my.finalizer");
/// assert!(has_finalizer(&pod, "my.finalizer"));
/// ```
pub fn has_finalizer<K: Resource>(obj: &K, name: &str) -> bool {
    obj.finalizers().iter().any(|f| f == name)
}

/// Adds a finalizer to `obj` if it is not already present
///
/// Returns `true` if the finalizer was added, `false` if it was already present.
///
/// This only changes `obj` in memory. For reconcilers, prefer `kube_runtime::finalizer`, which
/// persists the finalizer before any cleanup-relevant work starts (see the [module docs](self)).
pub fn add_finalizer<K: Resource>(obj: &mut K, name: &str) -> bool {
    if has_finalizer(obj, name) {
        return false;
    }
    obj.finalizers_mut().push(name.to_string());
    true
}

/// Removes a finalizer from `obj` if present
///
/// Returns `true` if the finalizer was removed, `false` if it was not present.
pub fn remove_finalizer<K: Resource>(obj: &mut K, name: &str) -> bool {
    let finalizers = obj.finalizers_mut();
    let len_before = finalizers.len();
    finalizers.retain(|f| f != name);
    finalizers.len() != len_before
}

#[cfg(test)]
mod tests {
    use super::{add_finalizer, has_finalizer, remove_finalizer};
    use crate::ResourceExt;
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn finalizer_add_is_idempotent() {
        let mut pod = Pod::default();
        assert!(!has_finalizer(&pod, "my.finalizer"));
        assert!(add_finalizer(&mut pod, "my.finalizer"));
        assert!(has_finalizer(&pod, "my.finalizer"));
        // Adding again is a no-op
        assert!(!add_finalizer(&mut pod, "my.finalizer"));
        assert_eq!(pod.finalizers(), &["my.finalizer".to_string()]);
    }

    #[test]
    fn finalizer_remove_reports_whether_anything_changed() {
        let mut pod = Pod::default();
        add_finalizer(&mut pod, "my.finalizer");
        assert!(remove_finalizer(&mut pod, "my.finalizer"));
        assert!(!has_finalizer(&pod, "my.finalizer"));
        // Removing again is a no-op
        assert!(!remove_finalizer(&mut pod, "my.finalizer"));
    }
}
