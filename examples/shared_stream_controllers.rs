use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Pod, PodCondition};
use kube::{
    api::{Patch, PatchParams},
    runtime::{controller::Action, reflector, watcher, Config, Controller, WatchStreamExt},
    Api, Client, ResourceExt,
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use thiserror::Error;

// Helper module that namespaces two constants describing a Kubernetes status condition
pub mod condition {
    pub static UNDOCUMENTED_TYPE: &str = "UndocumentedPort";
    pub static STATUS_TRUE: &str = "True";
}

const SUBSCRIBE_BUFFER_SIZE: usize = 256;

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to patch pod: {0}")]
    WriteFailed(#[source] kube::Error),

    #[error("Missing po field: {0}")]
    MissingField(&'static str),
}

#[derive(Clone)]
struct Data {
    client: Client,
}

/// A simple reconciliation function that will copy a pod's labels into the annotations.
async fn reconcile_metadata(pod: Arc<Pod>, ctx: Arc<Data>) -> Result<Action, Error> {
    if pod.name_any() == "kube-system" {
        return Ok(Action::await_change());
    }

    let labels = pod.labels();
    if labels.is_empty() {
        return Ok(Action::await_change());
    }

    let mut annotations = pod.annotations().clone();
    for (key, value) in labels {
        annotations.insert(key.to_owned(), value.to_owned());
    }

    let mut pod = (*pod).clone();
    pod.metadata.annotations = Some(annotations);
    pod.metadata.managed_fields = None;

    let pod_api = Api::<Pod>::namespaced(
        ctx.client.clone(),
        pod.metadata
            .namespace
            .as_ref()
            .ok_or_else(|| Error::MissingField(".metadata.name"))?,
    );

    pod_api
        .patch(
            &pod.name_any(),
            &PatchParams::apply("controller-1"),
            &Patch::Apply(&pod),
        )
        .await
        .map_err(Error::WriteFailed)?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

/// Another reconiliation function that will add an 'UndocumentedPort' condition to pods that do
/// do not have any ports declared across all containers.
async fn reconcile_status(pod: Arc<Pod>, ctx: Arc<Data>) -> Result<Action, Error> {
    for container in pod.spec.clone().unwrap_or_default().containers.iter() {
        if container.ports.clone().unwrap_or_default().len() != 0 {
            tracing::debug!(name = %pod.name_any(), "Skipped updating pod with documented ports");
            return Ok(Action::await_change());
        }
    }

    let pod_api = Api::<Pod>::namespaced(
        ctx.client.clone(),
        pod.metadata
            .namespace
            .as_ref()
            .ok_or_else(|| Error::MissingField(".metadata.name"))?,
    );

    let undocumented_condition = PodCondition {
        type_: condition::UNDOCUMENTED_TYPE.into(),
        status: condition::STATUS_TRUE.into(),
        ..Default::default()
    };
    let value = serde_json::json!({
        "status": {
            "name": pod.name_any(),
            "kind": "Pod",
            "conditions": vec![undocumented_condition]
        }
    });
    pod_api
        .patch_status(
            &pod.name_any(),
            &PatchParams::apply("controller-2"),
            &Patch::Strategic(value),
        )
        .await
        .map_err(Error::WriteFailed)?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let pods = Api::<Pod>::namespaced(client.clone(), "default");
    let config = Config::default().concurrency(2);
    let ctx = Arc::new(Data { client });

    // Create a shared store with a predefined buffer that will be shared between subscribers.
    let (reader, writer) = reflector::store_shared(SUBSCRIBE_BUFFER_SIZE);
    // Before threading an object watch through the store, create a subscriber.
    // Any number of subscribers can be created from one writer.
    let subscriber = writer
        .subscribe()
        .expect("subscribers can only be created from shared stores");

    // Reflect a stream of pod watch events into the store and apply a backoff. For subscribers to
    // be able to consume updates, the reflector must be shared.
    let mut pod_watch = watcher(pods.clone(), Default::default())
        .default_backoff()
        .reflect_shared(writer)
        .boxed();

    // Create the first controller using the reconcile_metadata function. Controllers accept
    // subscribers through a dedicated interface.
    let mut metadata_controller = Controller::for_shared_stream(subscriber.clone(), reader)
        .with_config(config.clone())
        .run(
            reconcile_metadata,
            |pod, error, _| {
                tracing::error!(%error, name = %pod.name_any(), "Failed to reconcile metadata");
                Action::requeue(Duration::from_secs(10))
            },
            ctx.clone(),
        )
        .boxed();

    // Subscribers can be used to get a read handle on the store, if the initial handle has been
    // moved or dropped.
    let reader = subscriber.reader();
    // Create the second controller using the reconcile_status function.
    let mut status_controller = Controller::for_shared_stream(subscriber, reader)
        .with_config(config)
        .run(
            reconcile_status,
            |pod, error, _| {
                tracing::error!(%error, name = %pod.name_any(), "Failed to reconcile status");
                Action::requeue(Duration::from_secs(10))
            },
            ctx,
        )
        .boxed();

    // A simple handler to shutdown on CTRL-C or SIGTERM.
    let mut shutdown_rx = shutdown_handler();

    // Drive streams to readiness. The initial watch (that is reflected) needs to be driven to
    // consume events from the API Server and forward them to subscribers.
    //
    // Both controllers will operate on shared objects.
    loop {
        tokio::select! {
            Some(res) = metadata_controller.next() => {
                match res {
                    Ok(v) => info!("Reconciled metadata {v:?}"),
                    Err(error) => warn!(%error, "Failed to reconcile metadata"),
                }
            },

            Some(res) = status_controller.next() => {
                match res {
                    Ok(v) => info!("Reconciled status {v:?}"),
                    Err(error) => warn!(%error, "Failed to reconcile object"),
                }
            },

            Some(item) = pod_watch.next() => {
                match item {
                    Err(error) => tracing::error!(%error, "Received error from main watcher stream"),
                    _ => {}
                }
            },

            _ = shutdown_rx.recv() => {
                tracing::info!("Received shutdown signal; terminating...");
                break;
            }
        }
    }

    Ok(())
}

// Create a channel that will hold at most one item. Whenever a signal is received it is sent
// through the channel.
// We do not use a oneshot because we don't want to clone the receiver in each loop iteration.
fn shutdown_handler() -> mpsc::Receiver<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("should not fail to register sighandler");
    let ctrlc = tokio::signal::ctrl_c();
    tokio::spawn(async move {
        tokio::select! {
            _ = terminate.recv() => {
                shutdown_tx.send(()).await
            },

            _ = ctrlc => {
                shutdown_tx.send(()).await
            }
        }
    });

    shutdown_rx
}
