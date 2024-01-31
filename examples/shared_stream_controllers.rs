use std::{sync::Arc, time::Duration};

use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Patch, PatchParams},
    core::ObjectMeta,
    runtime::{controller::Action, watcher, Config, Controller},
    Api, Client, ResourceExt,
};
use tracing::{info, warn};

use thiserror::Error;

#[derive(Clone)]
struct Data {
    client: Client,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let pods = Api::<Pod>::all(client.clone());

    // ?
    let config = Config::default().concurrency(2);
    let ctx = Arc::new(Data { client });

    //
    // Controller new returns Self. Each method consumes self and returns a new
    // Self
    //
    // new() creates a watcher, and uses it to then create a reflector. Moves it
    // all in the controller's memory.
    //
    // reconcile_all_on() takes a trigger (i.e. a stream). When the trigger
    // fires, it will reconcile _all_ managed objects. For us, it means the
    // trigger will be a stream element.
    //
    // shutdown_on_signal() is interesting to look at for research purposes
    //
    // run() is the equivalent of build(). Consumes everything and yields back a
    // stream. We'll need to dedicate some time to reviewing it.
    //
    // for_each() will run a closure on each stream element effectively
    // consuming the stream.
    // TODO: Document this builder pattern in my notes
    Controller::new(pods, watcher::Config::default())
        .with_config(config)
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
        })
        .await;

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
