use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::{Pod, PodCondition, PodStatus};
use kube::{
    api::{Patch, PatchParams},
    core::ObjectMeta,
    runtime::{controller::Action, reflector::store::Writer, watcher, Config, Controller, WatchStreamExt},
    Api, Client, ResourceExt,
};
use tokio::signal::unix::{signal, SignalKind};
use tracing::{info, info_span, warn, Instrument};

use thiserror::Error;

pub mod condition {
    pub static UNDOCUMENTED_TYPE: &str = "UndocumentedPort";
    pub static STATUS_TRUE: &str = "True";
}

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

    // (1): create a store
    let writer = Writer::<Pod>::new(Default::default());

    // (2): split the stream:
    //      - create a handle that can be cloned to get more readers
    //      - pass through events from root stream through a reflector
    //
    //  Note: if we wanted to, we could apply a backoff _before_ we spill into the reflector
    let (subscriber, reflector) = watcher(pods.clone(), Default::default()).reflect_shared(writer, 1);

    // (3): schedule the root stream with the runtime
    //      - apply a backoff to the root stream
    //      - poll it to handle errors
    //  scheduling with the runtime ensures the stream will be polled continously and allow
    //  readers to make progress.
    tokio::spawn(
        async move {
            // Pin on the heap so we don't overflow our stack
            // Put a backoff on it.
            // - Depending on how we want to handle backpressure, the backoff could help to relax
            //   the flow of data
            // i.e. the root stream has a buffer that objects get put into. When an object is in the
            // buffer, it is cloned and sent to all readers. Once all readers have acked their copy,
            // the item is removed from the buffer.
            //
            // A backoff here could ensure that when the buffer is full, we backpressure in the root
            // stream by not consuming watcher output. We give clients enough time to make progress and
            // ensure the next time the root stream is polled it can make progress by pushing into the
            // buffer.
            let mut reflector = reflector.default_backoff().boxed();
            tracing::info!("Polling root");
            while let Some(next) = reflector.next().await {
                match next {
                    Err(error) => tracing::error!(%error, "Received error from main watcher stream"),
                    _ => {}
                }
            }
        }
        .instrument(info_span!("root_stream")),
    );


    // Create metadata controller to edit annotations
    let reader = subscriber.reader();
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
    tokio::spawn(metadata_controller);

    // Create status controller
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
    tokio::spawn(status_controller);

    // Handle shutdown
    //
    // In a more nicely put together example we'd want to actually drain everything
    // instead of having controllers manage signals on their own
    //
    // The lack of a drain abstraction atm made me skip it but when the example is ready we should
    // consider handling shutdowns well to help users out
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        _ = interrupt.recv() => {
            info!("Received SIGINT; terminating...");
        },

        _ = terminate.recv() => {
            info!("Received SIGTERM; terminating...");
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
