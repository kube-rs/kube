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
    /// Get a namespaced resource
    pub async fn get_namespaced<K>(&self, name: &str, ns: &Namespace) -> Result<K>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.get_raw(request, &GetParams::default(), name).await
    }

    /// List a namespaced resource
    pub async fn list_namespaced<K>(&self, lp: &ListParams, ns: &Namespace) -> Result<ObjectList<K>>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = namespaced_request::<K>(ns);
        self.list_raw(request, lp).await
    }

    /// Get a cluster scoped resource
    pub async fn get<K>(&self, name: &str) -> Result<K>
    where
        K: Resource<Scope = ClusterResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = cluster_request::<K>();
        self.get_raw(request, &GetParams::default(), name).await
    }

    /// List a namespaced resource across namespaces
    pub async fn list_all<K>(&self, lp: &ListParams) -> Result<ObjectList<K>>
    where
        K: Resource<Scope = NamespaceResourceScope> + Serialize + DeserializeOwned + Clone + Debug,
        <K as Resource>::DynamicType: Default,
    {
        let request = global_request::<K>();
        self.list_raw(request, lp).await
    }

    /// Convenience helper to list namespaces
    pub async fn list_available_namespaces(&self, lp: &ListParams) -> Result<ObjectList<k8sNs>> {
        let request = cluster_request::<k8sNs>();
        self.list_raw(request, lp).await
    }
}

#[cfg(test)]
#[cfg(feature = "client")]
mod test {
    use super::{Client, ListParams};
    use kube_core::ResourceExt;

    #[tokio::test]
    #[ignore = "needs cluster (will list namespaces)"]
    async fn list_pods_across_namespaces() -> Result<(), Box<dyn std::error::Error>> {
        use k8s_openapi::api::core::v1::Pod;

        let client = Client::try_default().await?;
        let lp = ListParams::default();
        for ns in client.list_available_namespaces(&lp).await? {
            for p in client.list_namespaced::<Pod>(&lp, &(&ns).try_into()?).await? {
                println!("Found pod {} in {}", p.name_any(), ns.name_any());
            }
        }
        Ok(())
    }
}
