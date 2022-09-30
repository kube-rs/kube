//! Generic request impls on Client for Resource implementors
use crate::{Client, Error, Result};
use either::Either;
use futures::Stream;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use kube_core::{
    dynamic::{ApiResource, DynamicObject},
    gvk::{GroupVersionKind, GroupVersionResource},
    metadata::{ListMeta, ObjectMeta, TypeMeta},
    object::{NotUsed, Object, ObjectList},
    params::{
        DeleteParams, ListParams, Patch, PatchParams, PostParams, Preconditions, PropagationPolicy,
        ValidationDirective,
    },
    request::Request,
    response::Status,
    watch::WatchEvent,
    ClusterResourceScope, DynamicResourceScope, ErrorResponse, NamespaceResourceScope, Resource, ResourceExt,
    SubResourceScope,
};

/// Newtype wrapper for Namespace
///
/// TODO: deref and into?
pub struct Namespace(String);


// helper constructors for the Request object
fn namespaced_request<K>(ns: &Namespace) -> Request
where
    K: Resource<Scope = NamespaceResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), Some(&ns.0));
    Request::new(url)
}
fn cluster_request<K>() -> Request
where
    K: Resource<Scope = ClusterResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), None);
    Request::new(url)
}

fn dynamic_namespaced_request<K>(dyntype: &K::DynamicType, ns: &Namespace) -> Request
where
    K: Resource<Scope = DynamicResourceScope>,
{
    let url = K::url_path(dyntype, Some(&ns.0));
    Request::new(url)
}
fn dynamic_cluster_request<K>(dyntype: &K::DynamicType) -> Request
where
    K: Resource<Scope = DynamicResourceScope>,
{
    let url = K::url_path(dyntype, None);
    Request::new(url)
}

/// Unconstrained private helpers for any Resource implementor
impl Client {
    async fn create_raw<K>(&self, r: Request, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Resource + Serialize + DeserializeOwned + Clone + Debug,
    {
        let bytes = serde_json::to_vec(&data).map_err(Error::SerdeError)?;
        let mut req = r.create(pp, bytes).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create");
        self.request::<K>(req).await
    }

    async fn create_raw_subresource<K, S>(
        &self,
        r: Request,
        name: &str,
        pp: &PostParams,
        data: &S,
    ) -> Result<S>
    where
        K: Resource,
        S: Resource<Scope = SubResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        S::DynamicType: Default, // limited to static queries
    {
        let bytes = serde_json::to_vec(&data).map_err(Error::SerdeError)?;
        let subname = S::plural(&S::DynamicType::default()).to_string();
        let mut req = r
            .create_subresource(&subname, name, pp, bytes)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create_subresource");
        self.request::<S>(req).await
    }

    async fn delete_raw<K>(&self, r: Request, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>>
    where
        K: Resource + DeserializeOwned + Clone + Debug,
    {
        let mut req = r.delete(name, dp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("delete");
        self.request_status::<K>(req).await
    }
}

/// Methods for NamespaceResourceScope Resource implementors
impl Client {
    /// Create a namespaced resource
    pub async fn create_namespaced<K>(&self, ns: &Namespace, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.create_raw(request, pp, data).await
    }

    /// Delete a namespaced resource
    pub async fn delete_namespaced<K>(
        &self,
        name: &str,
        ns: &Namespace,
        dp: &DeleteParams,
    ) -> Result<Either<K, Status>>
    where
        K: Resource<Scope = NamespaceResourceScope> + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.delete_raw(request, name, dp).await
    }
}

/// Methods for ClusterResourceScope Resource implementors
impl Client {
    /// Create a cluster resource
    pub async fn create<K>(&self, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Resource<Scope = ClusterResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.create_raw(request, pp, data).await
    }

    /// Delete a cluster resource
    pub async fn delete<K>(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>>
    where
        K: Resource<Scope = ClusterResourceScope> + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.delete_raw(request, name, dp).await
    }
}

/// Methods for SubResourceScope
///
/// These methods are generic over two Resource types;
/// K: The root type the subresource is attached to (e.g. ServiceAccount)
/// S: The sub type sitting ontop of a resource (e.g. TokenReview)
impl Client {
    /// Create an arbitrary subresource under a resource
    pub async fn create_subresource<K, S>(&self, name: &str, pp: &PostParams, data: &S) -> Result<S>
    where
        K: Resource<Scope = ClusterResourceScope>,
        S: Resource<Scope = SubResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
        <S as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.create_raw_subresource::<K, S>(request, name, pp, data).await
    }

    /// Create an arbitrary namespaced subresource under a resource
    pub async fn create_namespaced_subresource<K, S>(
        &self,
        name: &str,
        ns: &Namespace,
        pp: &PostParams,
        data: &S,
    ) -> Result<S>
    where
        K: Resource<Scope = NamespaceResourceScope>,
        S: Resource<Scope = SubResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
        <S as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.create_raw_subresource::<K, S>(request, name, pp, data).await
    }
}


/// Methods for DynamicResourceScope
/// These resources can be Namespaced or Cluster scoped, so we implement both methods.
/// NB: We do not handle Dynamically scoped subresources at the moment
impl Client {
    /// Create a cluster resource
    pub async fn create_dyn<K>(&self, pp: &PostParams, data: &K, dt: &K::DynamicType) -> Result<K>
    where
        K: Resource<Scope = DynamicResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
    {
        let request = dynamic_cluster_request::<K>(dt);
        self.create_raw(request, pp, data).await
    }

    /// Delete a cluster resource
    pub async fn delete_with<K>(
        &self,
        name: &str,
        dp: &DeleteParams,
        dt: &K::DynamicType,
    ) -> Result<Either<K, Status>>
    where
        K: Resource<Scope = DynamicResourceScope> + DeserializeOwned + Clone + Debug,
    {
        let request = dynamic_cluster_request::<K>(dt);
        self.delete_raw(request, name, dp).await
    }

    /// Create a namespaced resource
    pub async fn create_namespaced_dyn<K>(
        &self,
        ns: &Namespace,
        pp: &PostParams,
        data: &K,
        dt: &K::DynamicType,
    ) -> Result<K>
    where
        K: Resource<Scope = DynamicResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
    {
        // TODO: need to block on wrong scope at runtime via DynamicType
        // but it's currently hidden in ApiCapabilities
        // See https://github.com/kube-rs/kube/issues/1036
        let request = dynamic_namespaced_request::<K>(dt, ns);
        self.create_raw(request, pp, data).await
    }

    /// Delete a namespaced resource
    pub async fn delete_namespaced_dyn<K>(
        &self,
        name: &str,
        ns: &Namespace,
        dp: &DeleteParams,
        dt: &K::DynamicType,
    ) -> Result<Either<K, Status>>
    where
        K: Resource<Scope = DynamicResourceScope> + DeserializeOwned + Clone + Debug,
    {
        // TODO: need to block on wrong scope at runtime via DynamicType
        // but it's currently hidden in ApiCapabilities
        // See https://github.com/kube-rs/kube/issues/1036
        let request = dynamic_namespaced_request::<K>(dt, ns);
        self.delete_raw(request, name, dp).await
    }
}
