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

// TODO: remove these i think they are not necessary
fn cluster_subresource_request<K, S>() -> Request
where
    K: Resource<Scope = ClusterResourceScope>,
    S: Resource<Scope = SubResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), None);
    Request::new(url)
}
fn namespaced_subresource_request<K, S>(ns: &Namespace) -> Request
where
    K: Resource<Scope = NamespaceResourceScope>,
    S: Resource<Scope = SubResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), Some(&ns.0));
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

    async fn create_raw_subresource<K>(
        &self,
        r: Request,
        name: &str,
        subresource_name: &str,
        pp: &PostParams,
        data: &K,
    ) -> Result<K>
    where
        K: Resource + Serialize + DeserializeOwned + Clone + Debug,
    {
        let bytes = serde_json::to_vec(&data).map_err(Error::SerdeError)?;
        // TODO: figure out why create_subresource needs a name, but create does not
        let mut req = r
            .create_subresource(subresource_name, name, pp, bytes)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create_subresource");
        self.request::<K>(req).await
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

/// Methods for DynamicResourceScope
/// These resources can be Namespaced or Cluster scoped, so we implement both methods.
impl Client {
    /// Create a cluster resource
    pub async fn create_with<K>(&self, pp: &PostParams, data: &K, dt: &K::DynamicType) -> Result<K>
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
    pub async fn create_namespaced_with<K>(
        &self,
        ns: &Namespace,
        pp: &PostParams,
        data: &K,
        dt: &K::DynamicType,
    ) -> Result<K>
    where
        K: Resource<Scope = DynamicResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
    {
        let request = dynamic_namespaced_request::<K>(dt, ns);
        self.create_raw(request, pp, data).await
    }

    /// Delete a namespaced resource
    pub async fn delete_namespaced_with<K>(
        &self,
        name: &str,
        ns: &Namespace,
        dp: &DeleteParams,
        dt: &K::DynamicType,
    ) -> Result<Either<K, Status>>
    where
        K: Resource<Scope = DynamicResourceScope> + DeserializeOwned + Clone + Debug,
    {
        let request = dynamic_namespaced_request::<K>(dt, ns);
        self.delete_raw(request, name, dp).await
    }
}


/// Methods for DynamicResourceScope
/// NB: Currently not handling Dynamically scoped subresources...
/// ...maybe this is a sign that Dynamic scopes are disjoint from scopes
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
        let subresource_name = S::plural(&S::DynamicType::default()).to_string();
        self.create_raw_subresource(request, &subresource_name, name, pp, data)
            .await
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
        // this is the subresource name in subresource scope
        let subresource_name = S::plural(&S::DynamicType::default()).to_string();
        self.create_raw_subresource(request, &subresource_name, name, pp, data)
            .await
    }
}
