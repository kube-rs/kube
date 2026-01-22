use futures::StreamExt;
use k8s_openapi::api::apps::v1::{Deployment, ReplicaSet};
use kube::{
    Client, ResourceExt,
    api::Api,
    runtime::{
        controller::{Action, Controller},
        prewarmed_reflector, reflector, watcher,
    },
};
use std::sync::Arc;
use thiserror::Error;
use tokio::time::Duration;
use tracing::*;

#[derive(Debug, Error)]
enum Error {}

struct Ctx {
    replicasets: reflector::Store<ReplicaSet>,
}

async fn reconcile(deployment: Arc<Deployment>, ctx: Arc<Ctx>) -> Result<Action, Error> {
    let name = deployment.name_any();
    let namespace = deployment.namespace().unwrap_or_default();

    // Find owned ReplicaSets from the prewarmed store
    let owned_rs: Vec<_> = ctx
        .replicasets
        .state()
        .into_iter()
        .filter(|rs| {
            rs.namespace().as_deref() == Some(namespace.as_str())
                && rs
                    .owner_references()
                    .iter()
                    .any(|oref| oref.kind == "Deployment" && oref.name == name)
        })
        .collect();

    let total_replicas: i32 = owned_rs
        .iter()
        .filter_map(|rs| Some(rs.status.as_ref()?.replicas))
        .sum();

    info!(
        "Reconciling Deployment {}/{}: {} ReplicaSets, {} total replicas",
        namespace,
        name,
        owned_rs.len(),
        total_replicas
    );

    Ok(Action::requeue(Duration::from_secs(300)))
}

fn error_policy(_obj: Arc<Deployment>, _error: &Error, _ctx: Arc<Ctx>) -> Action {
    Action::requeue(Duration::from_secs(5))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let deployments = Api::<Deployment>::all(client.clone());
    let replicasets = Api::<ReplicaSet>::all(client.clone());

    // Create a reflector store for ReplicaSets
    let (rs_reader, rs_writer) = reflector::store::<ReplicaSet>();

    // Prewarm the ReplicaSet reflector - this awaits until the store is synced
    info!("Syncing ReplicaSet store...");
    let rs_stream = prewarmed_reflector(
        rs_reader.clone(),
        rs_writer,
        watcher(replicasets, watcher::Config::default()),
    )
    .await;
    info!("ReplicaSet store synced with {} entries", rs_reader.state().len());

    let ctx = Arc::new(Ctx {
        replicasets: rs_reader,
    });

    info!("Starting deployment controller");
    Controller::new(deployments, watcher::Config::default())
        .owns_stream(rs_stream)
        .shutdown_on_signal()
        .run(reconcile, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("Reconciled {:?}", o),
                Err(e) => warn!("Reconcile failed: {}", e),
            }
        })
        .await;

    Ok(())
}
