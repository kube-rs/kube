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

/// Convenience newtype for a namespace
pub struct Namespace(String);

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

// helper constructors for the Request object
fn namespaced_request<K>(ns: &Namespace) -> Request
where
    K: Resource<Scope = NamespaceResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), Some(&ns.0));
    Request::new(url)
}
fn global_request<K>() -> Request
where
    K: Resource<Scope = NamespaceResourceScope>,
    <K as Resource>::DynamicType: Default,
{
    let url = K::url_path(&K::DynamicType::default(), None);
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

impl Client {
    async fn get_raw<K>(&self, r: Request, gp: &GetParams, name: &str) -> Result<K>
    where
        K: Resource + Serialize + DeserializeOwned + Clone + Debug,
    {
        let mut req = r.get(name, gp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get");
        self.request::<K>(req).await
    }

    async fn list_raw<K>(&self, r: Request, lp: &ListParams) -> Result<ObjectList<K>>
    where
        K: Resource + DeserializeOwned + Clone,
    {
        let mut req = r.list(lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("list");
        self.request::<ObjectList<K>>(req).await
    }
}

/// Client extensions to allow typed api calls without [`Api`]
impl Client {
    /// Get a cluster scoped resource
    ///
    /// ```no_run
    /// # use k8s_openapi::api::rbac::v1::ClusterRole;
    /// # use kube::{ResourceExt, api::GetParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let crole = client.get::<ClusterRole>("cluster-admin").await?;
    /// assert_eq!(crole.name_unchecked(), "cluster-admin");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<K>(&self, name: &str) -> Result<K>
    where
        K: Resource<Scope = ClusterResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.get_raw(request, &GetParams::default(), name).await
    }

    /// Get a namespaced resource
    ///
    /// ```no_run
    /// # use k8s_openapi::api::core::v1::Service;
    /// # use kube::{ResourceExt, api::GetParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let ns = "default".try_into()?;
    /// let svc = client.get_namespaced::<Service>("kubernetes", &ns).await?;
    /// assert_eq!(svc.name_unchecked(), "kubernetes");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_namespaced<K>(&self, name: &str, ns: &Namespace) -> Result<K>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.get_raw(request, &GetParams::default(), name).await
    }

    /// List a cluster resource
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
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list<K>(&self, lp: &ListParams) -> Result<ObjectList<K>>
    where
        K: Resource<Scope = ClusterResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.list_raw(request, lp).await
    }

    /// List a namespaced resource
    ///
    /// ```no_run
    /// # use k8s_openapi::api::core::v1::Service;
    /// # use kube::{ResourceExt, api::ListParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let lp = ListParams::default();
    /// let ns = "default".try_into()?;
    /// for svc in client.list_namespaced::<Service>(&lp, &ns).await? {
    ///     println!("Found service {}", svc.name_any());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_namespaced<K>(&self, lp: &ListParams, ns: &Namespace) -> Result<ObjectList<K>>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.list_raw(request, lp).await
    }

    /// List a namespaced resource across namespaces
    ///
    /// ```no_run
    /// # use k8s_openapi::api::batch::v1::Job;
    /// # use kube::{ResourceExt, api::ListParams};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: kube::Client = todo!();
    /// let lp = ListParams::default();
    /// for j in client.list_all::<Job>(&lp).await? {
    ///     println!("Found job {} in {}", j.name_any(), j.namespace().unwrap());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_all<K>(&self, lp: &ListParams) -> Result<ObjectList<K>>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = global_request::<K>();
        self.list_raw(request, lp).await
    }
}

#[cfg(test)]
#[cfg(feature = "client")]
mod test {
    use super::{Client, ListParams, Namespace};
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
        for ns in client.list::<k8sNs>(&lp).await? {
            // namespaced list
            for p in client.list_namespaced::<Pod>(&lp, &(&ns).try_into()?).await? {
                println!("Found pod {} in {}", p.name_any(), ns.name_any());
            }
        }
        // across-namespace list
        for j in client.list_all::<Job>(&lp).await? {
            println!("Found job {} in {}", j.name_any(), j.namespace().unwrap());
        }
        // namespaced get
        let default: Namespace = "default".try_into()?;
        let svc = client.get_namespaced::<Service>("kubernetes", &default).await?;
        assert_eq!(svc.name_unchecked(), "kubernetes");
        // global get
        let ca = client.get::<ClusterRole>("cluster-admin").await?;
        assert_eq!(ca.name_unchecked(), "cluster-admin");

        Ok(())
    }
}
