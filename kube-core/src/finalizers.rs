//! Generic helpers for managing finalizers on resources.
//!
//! These helpers only mutate the object in memory; persisting the change (e.g. via a patch) is
//! up to the caller. For reconcilers, prefer `kube_runtime::finalizer`
//! (<https://docs.rs/kube/latest/kube/runtime/finalizer/fn.finalizer.html>): it persists the
//! finalizer to the apiserver *before* any work starts, which adding it in memory and writing it
//! back later does not guarantee.
use crate::resource::{Resource, ResourceExt};

/// Extension trait for managing finalizers on any [`Resource`]
///
/// Kept separate from [`ResourceExt`] so that it is only in scope when imported explicitly, and
/// does not collide with downstream extension traits defining the same method names.
///
/// These methods only change the object in memory. For reconcilers, prefer
/// `kube_runtime::finalizer`, which persists the finalizer before any cleanup-relevant work starts
/// (see the [module docs](self)).
///
/// ```
/// use k8s_openapi::api::core::v1::Pod;
/// use kube_core::FinalizerExt;
///
/// let mut pod = Pod::default();
/// assert!(!pod.has_finalizer("my.finalizer"));
/// assert!(pod.add_finalizer("my.finalizer"));
/// assert!(pod.has_finalizer("my.finalizer"));
/// assert!(pod.remove_finalizer("my.finalizer"));
/// ```
pub trait FinalizerExt: Resource {
    /// Returns whether the given finalizer is present
    fn has_finalizer(&self, name: &str) -> bool;
    /// Adds a finalizer if it is not already present
    ///
    /// Returns `true` if the finalizer was added, `false` if it was already present.
    fn add_finalizer(&mut self, name: &str) -> bool;
    /// Removes a finalizer if present
    ///
    /// Returns `true` if the finalizer was removed, `false` if it was not present.
    fn remove_finalizer(&mut self, name: &str) -> bool;
}

impl<K: Resource> FinalizerExt for K {
    fn has_finalizer(&self, name: &str) -> bool {
        self.finalizers().iter().any(|f| f == name)
    }

    fn add_finalizer(&mut self, name: &str) -> bool {
        if self.has_finalizer(name) {
            return false;
        }
        self.finalizers_mut().push(name.to_string());
        true
    }

    fn remove_finalizer(&mut self, name: &str) -> bool {
        let finalizers = self.finalizers_mut();
        let len_before = finalizers.len();
        finalizers.retain(|f| f != name);
        finalizers.len() != len_before
    }
}

#[cfg(test)]
mod tests {
    use super::FinalizerExt;
    use crate::ResourceExt;
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn finalizer_add_is_idempotent() {
        let mut pod = Pod::default();
        assert!(!pod.has_finalizer("my.finalizer"));
        assert!(pod.add_finalizer("my.finalizer"));
        assert!(pod.has_finalizer("my.finalizer"));
        // Adding again is a no-op
        assert!(!pod.add_finalizer("my.finalizer"));
        assert_eq!(pod.finalizers(), &["my.finalizer".to_string()]);
    }

    #[test]
    fn finalizer_remove_reports_whether_anything_changed() {
        let mut pod = Pod::default();
        pod.add_finalizer("my.finalizer");
        assert!(pod.remove_finalizer("my.finalizer"));
        assert!(!pod.has_finalizer("my.finalizer"));
        // Removing again is a no-op
        assert!(!pod.remove_finalizer("my.finalizer"));
    }
}
