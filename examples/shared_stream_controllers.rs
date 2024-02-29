use std::{sync::Arc, time::Duration};

use futures::{Stream, StreamExt, TryStream, TryStreamExt};
use k8s_openapi::api::core::v1::{Pod, PodCondition, PodStatus};
use kube::{
    api::{Patch, PatchParams},
    core::ObjectMeta,
    runtime::{
        controller::Action,
        reflector::{shared_reflector, store::Writer},
        watcher, Config, Controller, WatchStreamExt,
    },
    Api, Client, ResourceExt,
};
use tracing::{info, warn};

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

    // ?
    let config = Config::default().concurrency(2);
    let ctx = Arc::new(Data { client });

    // (1): Create a store & have it transform the stream to return arcs
    let writer = Writer::<Pod>::new(Default::default());
    let reader = writer.as_reader();
    let root = watcher(pods.clone(), Default::default())
        .default_backoff()
        .reflect_shared(writer);
    let dup = root.subscribe().map(|obj| Ok(obj)).applied_objects();

    tokio::spawn(
        Controller::for_shared_stream(dup, reader, ())
            .with_config(config.clone())
            .shutdown_on_signal()
            .run(
                reconcile_metadata,
                |_, _, _| Action::requeue(Duration::from_secs(1)),
                ctx.clone(),
            )
            .for_each(|res| async move {
                match res {
                    Ok(v) => info!("reconciled {v:?}"),
                    Err(error) => warn!(%error, "failed to reconcile object"),
                }
            }),
    );

    // (2): we can't share streams yet so we just use the same primitives
    let writer2 = Writer::<Pod>::new(Default::default());
    let reader2 = writer2.as_reader();
    let reflector2 = shared_reflector(writer2, watcher(pods.clone(), Default::default()));

    Controller::for_shared_stream(root, reader2, ())
        .with_config(config)
        .shutdown_on_signal()
        .run(
            reconcile_status,
            |_, _, _| Action::requeue(Duration::from_secs(1)),
            ctx,
        )
        .for_each(|res| async move {
            match res {
                Ok(v) => info!("reconcile status for {v:?}"),
                Err(error) => warn!(%error, "failed to reconcile status for object"),
            }
        })
        .await;

    // (3): Figure out how to use the same store and create a shared stream from
    // the shared reflector :)

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
