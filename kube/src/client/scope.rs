//! Scopes delimiting which objects an API call applies to

use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope};

/// A scope for interacting with Kubernetes objects
pub trait Scope {
    /// The namespace associated with the [`Scope`], if any
    fn namespace(&self) -> Option<&str>;
}

/// Access all objects in the cluster (that the user has permission to access)
#[derive(Clone, Debug)]
pub struct ClusterScope;
/// Access all objects in one namespace (that the user has permission to access)
#[derive(Clone, Debug)]
pub struct NamespaceScope {
    /// Namespace that access is limited to
    pub namespace: String,
}
/// A [`Scope`] that is resolved at runtime
///
/// NOTE: By using [`DynamicScope`] you opt out of Kube's ability to validate that the scope is valid for a given operation
#[derive(Clone, Debug)]
pub enum DynamicScope {
    /// Access all objects in the cluster (that the user has permission to access)
    Cluster(ClusterScope),
    /// Access all objects in one namespace (that the user has permission to access)
    Namespace(NamespaceScope),
}

impl Scope for ClusterScope {
    fn namespace(&self) -> Option<&str> {
        None
    }
}
impl Scope for NamespaceScope {
    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }
}
impl Scope for DynamicScope {
    fn namespace(&self) -> Option<&str> {
        self.inner().namespace()
    }
}
impl DynamicScope {
    fn inner(&self) -> &dyn Scope {
        match self {
            DynamicScope::Cluster(scope) => scope,
            DynamicScope::Namespace(scope) => scope,
        }
    }
}

/// Scope where a [`Resource`]'s objects can be read from or written to
pub trait NativeScope<Kind>: Scope {}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> NativeScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource<Scope = ClusterResourceScope>> NativeScope<Kind> for ClusterScope {}
impl<Kind> NativeScope<Kind> for DynamicScope {}

/// Scope where a [`Resource`]'s objects can be listed from
pub trait ListScope<Kind>: Scope {}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> ListScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource> ListScope<Kind> for ClusterScope {}
impl<Kind> ListScope<Kind> for DynamicScope {}
