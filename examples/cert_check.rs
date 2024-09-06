use std::borrow::Cow;

use k8s_openapi::api::core::v1::{ConfigMap, Namespace as Ns};
use k8s_openapi::NamespaceResourceScope;
use kube::client::scope::Namespace;
use kube::{api::ObjectMeta, client::scope::Cluster, Client, Resource, ResourceExt};
use serde::{Deserialize, Serialize};
use tracing::*;

use thiserror::Error;

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to open client: {0}")]
    ClientSetup(#[source] kube::Error),
    #[error("Failed to list namespaces: {0}")]
    NamespaceList(#[source] kube::Error),
    #[error("Failed to get ConfigMap: {0}")]
    FetchFailed(#[from] kube::Error),
    #[error("Expected certificate key in ConfigMap: {0}")]
    MissingKey(#[from] serde_json::Error),
}

// Variant of ConfigMap that only accepts ConfigMaps with a CA certificate
// to demonstrate manual implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CaConfigMapManual {
    metadata: ObjectMeta,
    data: CaConfigMapData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CaConfigMapData {
    #[serde(rename = "ca.crt")]
    ca_crt: String,
}

// Variant of ConfigMap that only accepts ConfigMaps with a CA certificate
// with inherited resource implementation
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(inherit = ConfigMap)]
struct CaConfigMap {
    metadata: ObjectMeta,
    data: CaConfigMapData,
}

// Display of a manual implementation
impl Resource for CaConfigMapManual {
    type DynamicType = ();
    type Scope = NamespaceResourceScope;

    fn kind(&(): &Self::DynamicType) -> Cow<'_, str> {
        Cow::Borrowed("ConfigMap")
    }

    fn group(&(): &Self::DynamicType) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn version(&(): &Self::DynamicType) -> Cow<'_, str> {
        Cow::Borrowed("v1")
    }

    fn plural(&(): &Self::DynamicType) -> Cow<'_, str> {
        Cow::Borrowed("configmaps")
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}


#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await.map_err(Error::ClientSetup)?;
    let namespaces = client
        .list::<Ns>(&Default::default(), &Cluster)
        .await
        .map_err(Error::NamespaceList)?;

    for ns in namespaces {
        let _ca: ConfigMap = client
            .get("kube-root-ca.crt", &Namespace::from(ns.name_any()))
            .await?;
        let _ca: CaConfigMapManual = client
            .get("kube-root-ca.crt", &Namespace::from(ns.name_any()))
            .await?;
        let ca: CaConfigMap = client
            .get("kube-root-ca.crt", &Namespace::from(ns.name_any()))
            .await?;
        info!(
            "Found correct root ca config map in {}: {}",
            ns.name_any(),
            ca.name_any()
        );
    }

    Ok(())
}
