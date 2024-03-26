use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Pod, PodCondition, PodStatus};
use kube::{
    api::{Patch, PatchParams},
    core::ObjectMeta,
    runtime::{controller::Action, reflector::store::Writer, watcher, Config, Controller, WatchStreamExt},
    Api, Client, ResourceExt,
};
use tracing::{info, info_span, warn, Instrument};

use thiserror::Error;

pub mod condition {
    pub static UNDOCUMENTED_TYPE: &str = "UndocumentedPort";
    pub static STATUS_TRUE: &str = "True";
}

const SUBSCRIBE_BUFFER_SIZE: usize = 256;

#[derive(Clone)]
struct Data {
    client: Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let pods = Api::<Pod>::namespaced(client.clone(), "default");
    let config = Config::default().concurrency(2);
    let ctx = Arc::new(Data { client });

    // (1): create a store (with a dispatcher)
    let writer = Writer::<Pod>::new_with_dispatch(Default::default(), SUBSCRIBE_BUFFER_SIZE);
    // (2): create a subscriber
    let subscriber = writer.subscribe();
    // (2.5): create a watch stream
    let pod_watch = watcher(pods.clone(), Default::default())
        .default_backoff()
        .reflect_dispatch(writer);

    // (3): schedule the root (i.e. shared) stream with the runtime.
    //
    //  The runtime (tokio) will drive this task to readiness; the stream is
    //  polled continously and allows all downstream readers (i.e. subscribers)
    //  to make progress.
    tokio::spawn(async move {
        // Pin on the heap so we don't overflow our stack
        let mut watch = pod_watch.boxed();
        while let Some(next) = watch.next().await {
            // We are not interested in the returned events here, only in
            // handling errors.
            match next {
                Err(error) => tracing::error!(%error, "Received error from main watcher stream"),
                _ => {}
            }
        }
    });

    // (4): create a reader. We create a metadata controller that will mirror a
    // pod's labels as annotations.
    //
    // To create a controller that operates on a shared stream, we need two
    // handles:
    // - A handle to the store.
    // - A handle to a shared stream.
    //
    // The handle to the shared stream will be used to receive shared objects as
    // they are applied by the reflector.
    let reader = subscriber.reader();
    // Store readers can be created on-demand by calling `reader()` on a shared
    // stream handle. Stream handles are cheap to clone.
    let metadata_controller = Controller::for_shared_stream(subscriber.clone(), reader)
        .with_config(config.clone())
        .shutdown_on_signal()
        .run(
            reconcile_metadata,
            |_, _, _| Action::requeue(Duration::from_secs(1)),
            ctx.clone(),
        )
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile object"),
            }
        })
        .instrument(info_span!("metadata_controller"));

    // (5): Create status controller. Our status controller write a condition
    // whenever a pod has undocumented container ports (i.e. containers with no
    // exposed ports).
    //
    // This is the last controller we will create, so we can just move the
    // handle inside the controller.
    let reader = subscriber.reader();
    let status_controller = Controller::for_shared_stream(subscriber, reader)
        .with_config(config)
        .shutdown_on_signal()
        .run(
            reconcile_status,
            |_, _, _| Action::requeue(Duration::from_secs(1)),
            ctx,
        )
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("Reconciled {v:?}"),
                Err(error) => warn!(%error, "Failed to reconcile object"),
            }
        })
        .instrument(info_span!("status_controller"));

    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
    // (6): Last step, drive controllers to readiness. Controllers are futures
    // and need to be driven to make progress. A controller that's not driven
    // and operates on a subscribed stream will eventually block the shared stream.
    tokio::select! {
        _ = metadata_controller => {
        },

        _ = status_controller => {
        },

        _ = terminate.recv() => {
            info!("Received term signal; shutting down...")
        }

    }

    Ok(())
}

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to patch pod: {0}")]
    WriteFailed(#[source] kube::Error),

    #[error("Missing po field: {0}")]
    MissingField(&'static str),
}

/// Controller will trigger this whenever our main pod has changed. The function
/// reconciles a pod by copying over the labels to the annotations
async fn reconcile_metadata(pod: Arc<Pod>, ctx: Arc<Data>) -> Result<Action, Error> {
    if pod.name_any() == "kube-system" {
        return Ok(Action::requeue(Duration::from_secs(300)));
    }
    let labels = pod.metadata.labels.clone().unwrap_or_else(|| Default::default());
    if labels.len() == 0 {
        return Ok(Action::requeue(Duration::from_secs(180)));
    }

    let annotations = labels.clone();
    let p = Pod {
        metadata: ObjectMeta {
            name: Some(pod.name_any()),
            labels: Some(labels),
            annotations: Some(annotations),
            ..ObjectMeta::default()
        },
        spec: pod.spec.clone(),
        status: pod.status.clone(),
    };

    let pod_api = Api::<Pod>::namespaced(
        ctx.client.clone(),
        pod.metadata
            .namespace
            .as_ref()
            .ok_or_else(|| Error::MissingField(".metadata.name"))?,
    );

    pod_api
        .patch(
            &p.name_any(),
            &PatchParams::apply("controller-1"),
            &Patch::Apply(&p),
        )
        .await
        .map_err(Error::WriteFailed)?;

    Ok(Action::requeue(Duration::from_secs(300)))
}

/// Controller will trigger this whenever our main pod has changed. The
/// function reconciles a pod by writing a status if it does not document a
/// port.
async fn reconcile_status(pod: Arc<Pod>, ctx: Arc<Data>) -> Result<Action, Error> {
    let mut conditions = pod
        .status
        .clone()
        .unwrap_or_default()
        .conditions
        .unwrap_or_default();

    // If the condition already exists, exit
    for cond in conditions.iter() {
        if cond.type_ == condition::UNDOCUMENTED_TYPE {
            return Ok(Action::requeue(Duration::from_secs(300)));
        }
    }

    pod.spec
        .clone()
        .unwrap_or_default()
        .containers
        .iter()
        .for_each(|c| {
            if c.ports.clone().unwrap_or_default().len() == 0 {
                conditions.push(PodCondition {
                    type_: condition::UNDOCUMENTED_TYPE.into(),
                    status: condition::STATUS_TRUE.into(),
                    ..Default::default()
                })
            }
        });

    let mut current_conds = pod
        .status
        .clone()
        .unwrap_or_default()
        .conditions
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.type_ != condition::UNDOCUMENTED_TYPE && c.status != condition::STATUS_TRUE)
        .collect::<Vec<PodCondition>>();

    for condition in conditions {
        current_conds.push(condition);
    }

    let status = PodStatus {
        conditions: Some(current_conds),
        ..Default::default()
    };
    let pod_api = Api::<Pod>::namespaced(
        ctx.client.clone(),
        pod.metadata
            .namespace
            .as_ref()
            .ok_or_else(|| Error::MissingField(".metadata.name"))?,
    );

    let name = pod.name_any();
    let value = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "name": name,
            "status": status,
    });
    let p = Patch::Merge(value);
    pod_api
        .patch_status(&pod.name_any(), &PatchParams::apply("controller-2"), &p)
        .await
        .map_err(Error::WriteFailed)?;

    Ok(Action::requeue(Duration::from_secs(300)))
}
