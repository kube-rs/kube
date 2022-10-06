pub use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope, ResourceScope, SubResourceScope};

/// Getters for Scope
///
/// This allows getting information out of k8s-openapi::ResourceScope
/// without the need for specialization.
///
/// It also allows us to separate dynamic types from static ones.
pub trait Scope {
    /// Whether the Scope is namespaced
    fn is_namespaced() -> bool;
    /// Whether the Scope is a subresource
    fn is_subresource() -> bool;
    /// Whether the Scope is an indeteriminate dynamic scope
    fn is_dynamic() -> bool;
}

// extend the ResourceScope traits found in k8s-openapi

impl Scope for ClusterResourceScope {
    fn is_namespaced() -> bool {
        false
    }

    fn is_subresource() -> bool {
        false
    }

    fn is_dynamic() -> bool {
        false
    }
}
impl Scope for NamespaceResourceScope {
    fn is_namespaced() -> bool {
        true
    }

    fn is_subresource() -> bool {
        false
    }

    fn is_dynamic() -> bool {
        false
    }
}
impl Scope for SubResourceScope {
    fn is_namespaced() -> bool {
        false
    }

    fn is_subresource() -> bool {
        false
    }

    fn is_dynamic() -> bool {
        true
    }
}

/// Indicates that a [`Resource`] is of an indeterminate dynamic scope.
pub struct DynamicResourceScope {}
impl ResourceScope for DynamicResourceScope {}

// These implementations checks for namespace/subresource are false here
// because we cannot know the true scope from this struct alone.
// Refer to [`Resource::is_namespaced`] instead, which will inspect the
// DynamicType to find the discovered scope
impl Scope for DynamicResourceScope {
    fn is_namespaced() -> bool {
        false
    }

    fn is_subresource() -> bool {
        false
    }

    fn is_dynamic() -> bool {
        true
    }
}
