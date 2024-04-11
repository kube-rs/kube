use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Pod, PodCondition};
use kube::{
    api::{Patch, PatchParams},
    runtime::{controller::Action, reflector, watcher, Config, Controller, WatchStreamExt},
    Api, Client, ResourceExt,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

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

    let mut pod = (*pod).clone();
    pod.metadata.managed_fields = None;
    // combine labels and annotations into a new map
    let labels = pod.labels().clone().into_iter();
    pod.annotations_mut().extend(labels);

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
            debug!(name = %pod.name_any(), "Skipped updating pod with documented ports");
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

fn error_policy(obj: Arc<Pod>, error: &Error, _ctx: Arc<Data>) -> Action {
    error!(%error, name = %obj.name_any(), "Failed reconciliation");
    Action::requeue(Duration::from_secs(10))
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
    let pod_watch = watcher(pods.clone(), Default::default())
        .default_backoff()
        .reflect_shared(writer)
        .for_each(|res| async move {
            match res {
                Ok(event) => debug!("Received event on root stream {event:?}"),
                Err(error) => error!(%error, "Unexpected error when watching resource"),
            }
        });

    // Create the first controller using the reconcile_metadata function. Controllers accept
    // subscribers through a dedicated interface.
    let metadata_controller = Controller::for_shared_stream(subscriber.clone(), reader)
        .with_config(config.clone())
        .shutdown_on_signal()
        .run(reconcile_metadata, error_policy, ctx.clone())
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled metadata {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile metadata"),
            }
        });

    // Subscribers can be used to get a read handle on the store, if the initial handle has been
    // moved or dropped.
    let reader = subscriber.reader();
    // Create the second controller using the reconcile_status function.
    let status_controller = Controller::for_shared_stream(subscriber, reader)
        .with_config(config)
        .shutdown_on_signal()
        .run(reconcile_status, error_policy, ctx)
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled status {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile status"),
            }
        });

    // Drive streams to readiness. The initial watch (that is reflected) needs to be driven to
    // consume events from the API Server and forward them to subscribers.
    //
    // Both controllers will operate on shared objects.
    tokio::select! {
        _ = futures::future::join(metadata_controller, status_controller) => {},
        _ = pod_watch => {}
    }

    Ok(())
}
