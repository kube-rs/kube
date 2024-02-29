use crate::{Client, Error, Result};
use kube_core::{
    object::ObjectList,
    params::{GetParams, ListParams},
    request::Request,
    ClusterResourceScope, NamespaceResourceScope, Resource,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use k8s_openapi::api::core::v1::Namespace as k8sNs;

pub trait ListScope<K> {
    fn url_path(&self) -> String;
}

pub trait ObjectScope<K> {
    fn url_path(&self) -> String;
}

pub struct Cluster;

// All objects can be listed cluster-wide
impl<K> ListScope<K> for Cluster
where
    K: Resource,
    K::DynamicType: Default,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), None)
    }
}

// Only cluster-scoped objects can be named globally
impl<K> ObjectScope<K> for Cluster
where
    K: Resource<Scope = ClusterResourceScope>,
    K::DynamicType: Default,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), None)
    }
}

pub struct Namespace(String);

// Only namespaced objects can be accessed via namespace
impl<K> ListScope<K> for Namespace
where
    K: Resource<Scope = NamespaceResourceScope>,
    K::DynamicType: Default,
{
    fn url_path(&self) -> String {
        K::url_path(&K::DynamicType::default(), Some(&self.0))
    }
}

impl<K> ObjectScope<K> for Namespace
where
    K: Resource<Scope = NamespaceResourceScope>,
    K::DynamicType: Default,
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

/// Client extensions to allow typed api calls without [`Api`]
impl Client {
    /// Get a resource
    ///
    /// ```no_run
    /// # use k8s_openapi::api::rbac::v1::ClusterRole;
    /// # use kube::{ResourceExt, api::GetParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let crole = client.get::<ClusterRole>("cluster-admin", &Cluster).await?;
    /// assert_eq!(crole.name_unchecked(), "cluster-admin");
    /// let svc = client.get::<Service>("kubernetes", &Namespace::from("default")).await?;
    /// assert_eq!(svc.name_unchecked(), "kubernetes");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<K>(&self, name: &str, scope: &impl ObjectScope<K>) -> Result<K>
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

    /// List a resource
    ///
    /// ```no_run
    /// # use k8s_openapi::api::rbac::v1::ClusterRole;
    /// # use kube::{ResourceExt, api::ListParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let lp = ListParams::default();
    /// for svc in client.list::<ClusterRole>(&lp).await? {
    ///     println!("Found clusterrole {}", svc.name_any());
    /// }
    /// for svc in client.list::<Service>(&lp, &Namespace::from("default")).await? {
    ///     println!("Found service {}", svc.name_any());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list<K>(&self, lp: &ListParams, scope: &impl ListScope<K>) -> Result<ObjectList<K>>
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
#[cfg(feature = "client")]
mod test {
    use super::{Client, Cluster, ListParams, Namespace};
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
