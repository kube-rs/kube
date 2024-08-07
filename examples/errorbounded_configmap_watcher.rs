use std::borrow::Cow;

use futures::prelude::*;
use k8s_openapi::{api::core::v1::Pod, NamespaceResourceScope};
use kube::{
    api::{Api, ObjectMeta, ResourceExt},
    core::DeserializeGuard,
    runtime::{reflector::ObjectRef, watcher, WatchStreamExt},
    Client, Resource,
};
use serde::Deserialize;
use tracing::*;

// Variant of ConfigMap that only accepts ConfigMaps with a CA certificate
// to demonstrate parsing failure
#[derive(Deserialize, Debug, Clone)]
struct CaConfigMap {
    metadata: ObjectMeta,
    data: CaConfigMapData,
}

#[derive(Deserialize, Debug, Clone)]
struct CaConfigMapData {
    #[serde(rename = "ca.crt")]
    ca_crt: String,
}

// Normally you would derive this, but ConfigMap doesn't follow the standard spec/status pattern
impl Resource for CaConfigMap {
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
    let api = Api::<DeserializeGuard<CaConfigMap>>::default_namespaced(client);
    let use_watchlist = std::env::var("WATCHLIST").map(|s| s == "1").unwrap_or(false);
    let wc = if use_watchlist {
        // requires WatchList feature gate on 1.27 or later
        watcher::Config::default().streaming_lists()
    } else {
        watcher::Config::default()
    };

    watcher(api, wc)
        .applied_objects()
        .default_backoff()
        .try_for_each(|cm| async move {
            info!("saw {}", ObjectRef::from_obj(&cm));
            match cm.0 {
                Ok(cm) => info!("contents: {cm:?}"),
                Err(err) => warn!("failed to parse: {err}"),
            }
            Ok(())
        })
        .await?;
    Ok(())
}
