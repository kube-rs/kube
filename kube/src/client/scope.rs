//! Scopes delimiting which objects an API call applies to

use std::ops::Deref;

use k8s_openapi::{ClusterResourceScope, NamespaceResourceScope};
use kube_core::{subresource::Scale, DynamicObject, Resource};

/// A scope for interacting with Kubernetes objects
pub trait Scope {
    /// The namespace associated with the [`Scope`], if any
    fn namespace(&self) -> Option<&str>;
}
pub trait TopLevelScope: Scope {}

/// Access all objects in the cluster (that the user has permission to access)
#[derive(Clone, Debug)]
pub struct ClusterScope;
/// Access all objects in one namespace (that the user has permission to access)
#[derive(Clone, Debug)]
pub struct NamespaceScope {
    /// Namespace that access is limited to
    pub namespace: String,
}
#[derive(Clone, Debug)]
pub struct SubresourceScope<Parent: TopLevelScope, Kind: Resource> {
    pub parent: Parent,
    pub dyn_type: Kind::DynamicType,
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
    Subresource(Box<SubresourceScope<DynamicScope, DynamicObject>>),
}

impl Scope for ClusterScope {
    fn namespace(&self) -> Option<&str> {
        None
    }
}
impl TopLevelScope for ClusterScope {}
impl Scope for NamespaceScope {
    fn namespace(&self) -> Option<&str> {
        Some(&self.namespace)
    }
}
impl TopLevelScope for NamespaceScope {}
impl<Parent: TopLevelScope, Kind: Resource> self::Scope for SubresourceScope<Parent, Kind> {
    fn namespace(&self) -> Option<&str> {
        self.parent.namespace()
    }
}
impl Scope for DynamicScope {
    fn namespace(&self) -> Option<&str> {
        self.inner().namespace()
    }
}
impl TopLevelScope for DynamicScope {}
impl DynamicScope {
    fn inner(&self) -> &dyn Scope {
        match self {
            DynamicScope::Cluster(scope) => scope,
            DynamicScope::Namespace(scope) => scope,
            DynamicScope::Subresource(scope) => scope.deref(),
        }
    }

    pub(crate) fn of_object<T: Resource>(object: &T) -> Self {
        match &object.meta().namespace {
            Some(ns) => DynamicScope::Namespace(NamespaceScope {
                namespace: ns.to_string(),
            }),
            None => DynamicScope::Cluster(ClusterScope),
        }
    }
}

/// Scope where a [`Resource`]'s objects can be listed from
pub trait ResourceScope<Kind>: Scope {
    fn group(&self) -> &str;
    fn version(&self) -> &str;
    fn resource(&self) -> &str;
}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> ResourceScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource> ResourceScope<Kind> for ClusterScope {}
impl ResourceScope<Scale> for SubresourceScope<NamespaceScope, k8s_openapi::api::core::v1::Pod> {}
impl<Kind> ResourceScope<Kind> for DynamicScope {}

/// Scope where a [`Resource`]'s objects can be read from or written to
pub trait NativeScope<Kind>: ResourceScope<Kind> {}
impl<Kind: k8s_openapi::Resource<Scope = NamespaceResourceScope>> NativeScope<Kind> for NamespaceScope {}
impl<Kind: k8s_openapi::Resource<Scope = ClusterResourceScope>> NativeScope<Kind> for ClusterScope {}
impl<Kind> NativeScope<Kind> for DynamicScope {}
