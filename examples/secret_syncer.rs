// Demonstrates a controller some outside resource that it needs to clean up when the owner is deleted

// NOTE: This is designed to demonstrate how to use finalizers, but is not in itself a good use case for them.
// If you actually want to clean up other Kubernetes objects then you should use `ownerReferences` instead and let
// k8s garbage collect the children.
use futures::StreamExt;
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube::{
    api::{Api, DeleteParams, ListParams, ObjectMeta, Patch, PatchParams, Resource},
    error::ErrorResponse,
    runtime::{
        controller::{Action, Controller},
        finalizer::{finalizer, Event},
    },
};
use std::{sync::Arc, time::Duration};
use thiserror::Error;
use tracing::*;

#[derive(Debug, Error)]
enum Error {
    #[error("NoName")]
    NoName,
    #[error("NoNamespace")]
    NoNamespace,
    #[error("UpdateSecret: {0}")]
    UpdateSecret(#[source] kube::Error),
    #[error("DeleteSecret: {0}")]
    DeleteSecret(#[source] kube::Error),
}
type Result<T, E = Error> = std::result::Result<T, E>;

fn secret_name_for_configmap(cm: &ConfigMap) -> Result<String> {
    let name = cm.metadata.name.as_deref().ok_or(Error::NoName)?;
    Ok(format!("cmsyncer-{}", name))
}

async fn apply(cm: Arc<ConfigMap>, secrets: &kube::Api<Secret>) -> Result<Action> {
    info!("Reconciling {:?}", cm);
    let secret_name = secret_name_for_configmap(&cm)?;
    secrets
        .patch(
            &secret_name,
            &PatchParams::apply("configmap-secret-syncer.nullable.se"),
            &Patch::Apply(Secret {
                metadata: ObjectMeta {
                    name: Some(secret_name.clone()),
                    ..ObjectMeta::default()
                },
                string_data: cm.data.clone(),
                data: cm.binary_data.clone(),
                ..Secret::default()
            }),
        )
        .await
        .map_err(Error::UpdateSecret)?;
    Ok(Action::await_change())
}

async fn cleanup(cm: Arc<ConfigMap>, secrets: &kube::Api<Secret>) -> Result<Action> {
    info!("Cleaning up {:?}", cm);
    secrets
        .delete(&secret_name_for_configmap(&cm)?, &DeleteParams::default())
        .await
        .map(|_| ())
        .or_else(|err| match err {
            // Object is already deleted
            kube::Error::Api(ErrorResponse { code: 404, .. }) => Ok(()),
            err => Err(err),
        })
        .map_err(Error::DeleteSecret)?;
    Ok(Action::await_change())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;
    Controller::new(
        Api::<ConfigMap>::all(client.clone()),
        ListParams::default().labels("configmap-secret-syncer.nullable.se/sync=true"),
    )
    .run(
        |cm, _| {
            let ns = cm.meta().namespace.as_deref().ok_or(Error::NoNamespace).unwrap();
            let cms: Api<ConfigMap> = Api::namespaced(client.clone(), ns);
            let secrets: Api<Secret> = Api::namespaced(client.clone(), ns);
            async move {
                finalizer(
                    &cms,
                    "configmap-secret-syncer.nullable.se/cleanup",
                    cm,
                    |event| async {
                        match event {
                            Event::Apply(cm) => apply(cm, &secrets).await,
                            Event::Cleanup(cm) => cleanup(cm, &secrets).await,
                        }
                    },
                )
                .await
            }
        },
        |_obj, _err, _| Action::requeue(Duration::from_secs(2)),
        Arc::new(()),
    )
    .for_each(|msg| async move { info!("Reconciled: {:?}", msg) })
    .await;
    Ok(())
}
