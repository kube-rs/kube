use std::borrow::Cow;

use k8s_openapi::{
    api::core::v1::{ConfigMap, Namespace as Ns},
    NamespaceResourceScope,
};
use kube::{
    api::ObjectMeta,
    client::scope::{Cluster, Namespace},
    Client, Resource,
};
use serde::{Deserialize, Serialize};
use tracing::*;

// Our own way of representing data - partially typed in 2 ways
// For a ConfigMap variant that only accepts CA certificates
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CaConfigMapData {
    #[serde(rename = "ca.crt")]
    ca_crt: String,
}

// Method 1 :: inherit resource implementation from k8s_openapi's ConfigMap
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[resource(inherit = ConfigMap)]
struct CaConfigMap {
    metadata: ObjectMeta,
    data: CaConfigMapData,
}

// Method 2 :: manual Resource implementation
#[derive(Serialize, Deserialize, Debug, Clone)]
struct CaConfigMapManual {
    metadata: ObjectMeta,
    data: CaConfigMapData,
}
// Method 2 :: manual Resource implementation
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
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let namespaces = client.list::<Ns>(&Default::default(), &Cluster).await?;
    let kube_root = "kube-root-ca.crt";

    for ns in namespaces {
        let ns = Namespace::try_from(&ns)?;
        // Equivalent ways to GET using different structs and different Resource impls, with added field validation on top.
        let ca1: ConfigMap = client.get(kube_root, &ns).await?;
        let ca2: CaConfigMapManual = client.get(kube_root, &ns).await?;
        let ca3: CaConfigMap = client.get(kube_root, &ns).await?;
        info!("Found {kube_root} in {ns:?} with all 3 methods");
        debug!("ca1: {ca1:?}");
        debug!("ca2: {ca2:?}");
        debug!("ca3: {ca3:?}");
    }

    Ok(())
}
