//! Test helper for kube.
use std::time::Duration;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::{
    api::core::v1::ServiceAccount,
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube::{
    api::{ListParams, WatchEvent},
    Api, Client, CustomResourceExt, Resource,
};
use thiserror::Error;
use tokio::time;

pub mod k3d;

/// Test helper errors.
#[derive(Debug, Error)]
pub enum Error {
    #[error("timed out waiting for cluster readiness")]
    ClusterNotReady,
    #[error("failed to create CRD")]
    CreateCustomResourceDefinition(#[source] kube::Error),
    #[error("failed to watch CRD")]
    WatchCustomResourceDefinition(#[source] kube::Error),
    #[error("failed to resolve CRD watch event")]
    ResolveCustomResourceDefinitionWatchEvent(#[source] kube::Error),
    #[error("timed out waiting for CRD Established condition")]
    CustomResourceDefinitionNotEstablished,
}

/// Wait until the cluster is actually usable by making sure the default SA exists.
pub async fn cluster_ready(client: Client, timeout: u64) -> Result<(), Error> {
    time::timeout(Duration::from_secs(timeout), async move {
        tracing::info!("cluster: waiting for readiness");
        let mut interval = time::interval(Duration::from_secs(1));
        let sas: Api<ServiceAccount> = Api::default_namespaced(client);
        loop {
            interval.tick().await;
            if sas.get("default").await.is_ok() {
                break;
            }
        }
        tracing::info!("cluster: ready");
    })
    .await
    .map_err(|_| Error::ClusterNotReady)
}

/// Create CRD `K` and wait for `Established` condition.
pub async fn create_crd<K>(client: Client, timeout_secs: u32) -> Result<CustomResourceDefinition, Error>
where
    K: Resource<DynamicType = ()> + CustomResourceExt,
{
    tracing::info!("CRD: adding and waiting for Established condition");
    tracing::debug!("CRD: creating");
    let crds = Api::<CustomResourceDefinition>::all(client);
    crds.create(&Default::default(), &<K as CustomResourceExt>::crd())
        .await
        .map_err(Error::CreateCustomResourceDefinition)?;
    tracing::debug!("CRD: created");

    let lp = ListParams::default()
        .fields(&format!(
            "metadata.name={}.{}",
            <K as Resource>::plural(&()),
            <K as Resource>::group(&())
        ))
        .timeout(timeout_secs);
    let mut stream = crds
        .watch(&lp, "0")
        .await
        .map_err(Error::WatchCustomResourceDefinition)?
        .boxed_local();

    while let Some(status) = stream
        .try_next()
        .await
        .map_err(Error::ResolveCustomResourceDefinitionWatchEvent)?
    {
        match status {
            WatchEvent::Added(crd) => {
                tracing::debug!("CRD: added");
                tracing::trace!(
                    "CRD: conditions {:?}",
                    crd.status
                        .as_ref()
                        .map(|s| AsRef::<Vec<_>>::as_ref(&s.conditions))
                );
            }

            WatchEvent::Modified(crd) => {
                tracing::debug!("CRD: modified");
                tracing::trace!(
                    "CRD: conditions {:?}",
                    crd.status
                        .as_ref()
                        .map(|s| AsRef::<Vec<_>>::as_ref(&s.conditions))
                );
                let established = crd
                    .status
                    .as_ref()
                    .map(|s| {
                        s.conditions
                            .iter()
                            .any(|c| c.type_ == "Established" && c.status == "True")
                    })
                    .unwrap_or(false);
                if established {
                    tracing::info!("CRD: condition met");
                    return Ok(crd);
                }
            }

            WatchEvent::Deleted(_) => unreachable!("should never get deleted here"),

            WatchEvent::Bookmark(_) => {
                tracing::debug!("CRD: bookmark");
            }

            WatchEvent::Error(err) => {
                tracing::error!("CRD: {}", err);
            }
        }
    }

    Err(Error::CustomResourceDefinitionNotEstablished)
}
