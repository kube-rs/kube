use crate::{Client, Error, Result};
use k8s_openapi::api::core::v1::Namespace as k8sNs;
use kube_core::{
    object::ObjectList,
    params::{GetParams, ListParams},
    request::Request,
    ClusterResourceScope, DynamicResourceScope, NamespaceResourceScope, Resource,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// A marker trait to indicate cluster-wide operations are available
trait ClusterScope {}
/// A marker trait to indicate namespace-scoped operations are available
trait NamespaceScope {}

// k8s_openapi scopes get implementations for free
impl ClusterScope for ClusterResourceScope {}
impl NamespaceScope for NamespaceResourceScope {}
// our DynamicResourceScope can masquerade as either
impl NamespaceScope for DynamicResourceScope {}
impl ClusterScope for DynamicResourceScope {}

/// How to get the url for a collection
///
/// Pick one of `kube::client::Cluster` or `kube::client::Namespace`.
pub trait CollectionUrl<K> {
    fn url_path(&self) -> String;
}

/// How to get the url for an object
///
/// Pick one of `kube::client::Cluster` or `kube::client::Namespace`.
pub trait ObjectUrl<K> {
    fn url_path(&self) -> String;
}

/// Marker type for cluster level queries
pub struct Cluster;
/// Namespace newtype for namespace level queries
///
/// You can create this directly, or convert `From` a `String` / `&str`, or `TryFrom` an `k8s_openapi::api::core::v1::Namespace`
pub struct Namespace(String);

/// Scopes for `unstable-client` [`Client#impl-Client`] extension methods
pub mod scope {
    pub use super::{Cluster, Namespace};
}

// All objects can be listed cluster-wide
impl<K> CollectionUrl<K> for Cluster
where
    K: Resource,
    K::DynamicType: Default,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), None)
    }
}

// Only cluster-scoped objects can be named globally
impl<K> ObjectUrl<K> for Cluster
where
    K: Resource,
    K::DynamicType: Default,
    K::Scope: ClusterScope,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), None)
    }
}

// Only namespaced objects can be accessed via namespace
impl<K> CollectionUrl<K> for Namespace
where
    K: Resource,
    K::DynamicType: Default,
    K::Scope: NamespaceScope,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), Some(&self.0))
    }
}

impl<K> ObjectUrl<K> for Namespace
where
    K: Resource,
    K::DynamicType: Default,
    K::Scope: NamespaceScope,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), Some(&self.0))
    }
}

// can be created from a complete native object
impl TryFrom<&k8sNs> for Namespace {
    type Error = NamespaceError;

    fn try_from(ns: &k8sNs) -> Result<Namespace, Self::Error> {
        if let Some(n) = &ns.meta().name {
            Ok(Namespace(n.to_owned()))
        } else {
            Err(NamespaceError::MissingName)
        }
    }
}
// and from literals + owned strings
impl From<&str> for Namespace {
    fn from(ns: &str) -> Namespace {
        Namespace(ns.to_owned())
    }
}
impl From<String> for Namespace {
    fn from(ns: String) -> Namespace {
        Namespace(ns)
    }
}

#[derive(thiserror::Error, Debug)]
/// Failures to infer a namespace
pub enum NamespaceError {
    /// MissingName
    #[error("Missing Namespace Name")]
    MissingName,
}

/// Generic client extensions for the `unstable-client` feature
///
/// These methods allow users to query across a wide-array of resources without needing
/// to explicitly create an [`Api`](crate::Api) for each one of them.
///
/// ## Usage
/// 1. Create a [`Client`]
/// 2. Specify the [`scope`] you are querying at via [`Cluster`] or [`Namespace`] as args
/// 3. Specify the resource type you are using for serialization (e.g. a top level k8s-openapi type)
///
/// ## Example
///
/// ```no_run
/// # use k8s_openapi::api::core::v1::Pod;
/// # use k8s_openapi::api::core::v1::Service;
/// # use kube::client::scope::{Cluster, Namespace};
/// # use kube::{ResourceExt, api::ListParams};
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let client: kube::Client = todo!();
/// let lp = ListParams::default();
/// // List at Cluster level for Pod resource:
/// for pod in client.list::<Pod>(&lp, &Cluster).await? {
///     println!("Found pod {} in {}", pod.name_any(), pod.namespace().unwrap());
/// }
/// // Namespaced Get for Service resource:
/// let svc = client.get::<Service>("kubernetes", &Namespace::from("default")).await?;
/// assert_eq!(svc.name_unchecked(), "kubernetes");
/// # Ok(())
/// # }
/// ```
impl Client {
    /// Get a single instance of a `Resource` implementing type `K` at the specified scope.
    ///
    /// ```no_run
    /// # use k8s_openapi::api::rbac::v1::ClusterRole;
    /// # use k8s_openapi::api::core::v1::Service;
    /// # use kube::client::scope::{Cluster, Namespace};
    /// # use kube::{ResourceExt, api::GetParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let cr = client.get::<ClusterRole>("cluster-admin", &Cluster).await?;
    /// assert_eq!(cr.name_unchecked(), "cluster-admin");
    /// let svc = client.get::<Service>("kubernetes", &Namespace::from("default")).await?;
    /// assert_eq!(svc.name_unchecked(), "kubernetes");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<K>(&self, name: &str, scope: &impl ObjectUrl<K>) -> Result<K>
    where
        K: Resource + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let mut req = Request::new(scope.url_path())
            .get(name, &GetParams::default())
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get");
        self.request::<K>(req).await
    }

    /// List instances of a `Resource` implementing type `K` at the specified scope.
    ///
    /// ```no_run
    /// # use k8s_openapi::api::core::v1::Pod;
    /// # use k8s_openapi::api::core::v1::Service;
    /// # use kube::client::scope::{Cluster, Namespace};
    /// # use kube::{ResourceExt, api::ListParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let lp = ListParams::default();
    /// for pod in client.list::<Pod>(&lp, &Cluster).await? {
    ///     println!("Found pod {} in {}", pod.name_any(), pod.namespace().unwrap());
    /// }
    /// for svc in client.list::<Service>(&lp, &Namespace::from("default")).await? {
    ///     println!("Found service {}", svc.name_any());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list<K>(&self, lp: &ListParams, scope: &impl CollectionUrl<K>) -> Result<ObjectList<K>>
    where
        K: Resource + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let mut req = Request::new(scope.url_path())
            .list(lp)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("list");
        self.request::<ObjectList<K>>(req).await
    }
}

#[cfg(test)]
mod test {
    use super::{
        scope::{Cluster, Namespace},
        Client, ListParams,
    };
    use kube_core::ResourceExt;

    #[tokio::test]
    #[ignore = "needs cluster (will list/get namespaces, pods, jobs, svcs, clusterroles)"]
    async fn client_ext_list_get_pods_svcs() -> Result<(), Box<dyn std::error::Error>> {
        use k8s_openapi::api::{
            batch::v1::Job,
            core::v1::{Namespace as k8sNs, Pod, Service},
            rbac::v1::ClusterRole,
        };

        let client = Client::try_default().await?;
        let lp = ListParams::default();
        // cluster-scoped list
        for ns in client.list::<k8sNs>(&lp, &Cluster).await? {
            // namespaced list
            for p in client.list::<Pod>(&lp, &Namespace::try_from(&ns)?).await? {
                println!("Found pod {} in {}", p.name_any(), ns.name_any());
            }
        }
        // across-namespace list
        for j in client.list::<Job>(&lp, &Cluster).await? {
            println!("Found job {} in {}", j.name_any(), j.namespace().unwrap());
        }
        // namespaced get
        let default: Namespace = "default".into();
        let svc = client.get::<Service>("kubernetes", &default).await?;
        assert_eq!(svc.name_unchecked(), "kubernetes");
        // global get
        let ca = client.get::<ClusterRole>("cluster-admin", &Cluster).await?;
        assert_eq!(ca.name_unchecked(), "cluster-admin");

        Ok(())
    }
}
