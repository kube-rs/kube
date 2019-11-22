#[macro_use]
extern crate log;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
use kube::{
    api::{Api, Informer, Object, WatchEvent},
    client::APIClient,
    config,
};
use std::env;
type Pod = Object<PodSpec, PodStatus>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Pod(client.clone()).within(&namespace);
    let inf = Informer::new(resource.clone()).init().await?;

    // Here we both poll and reconcile based on events from the main thread
    // If you run this next to actix-web (say), spawn a thread and pass `inf` as app state
    loop {
        let mut pods = inf.poll().await?.boxed();

        // Handle events one by one, draining the informer
        while let Some(Ok(event)) = pods.next().await {
            handle_node(&resource, event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_node(_pods: &Api<Pod>, ev: WatchEvent<Pod>) -> anyhow::Result<()> {
    match ev {
        WatchEvent::Added(o) => {
            let containers = o
                .spec
                .containers
                .into_iter()
                .map(|c| c.name)
                .collect::<Vec<_>>();
            info!(
                "Added Pod: {} (containers={:?})",
                o.metadata.name, containers
            );
        }
        WatchEvent::Modified(o) => {
            let phase = o.status.unwrap().phase.unwrap();
            let owner = &o.metadata.ownerReferences[0];
            info!(
                "Modified Pod: {} (phase={}, owner={})",
                o.metadata.name, phase, owner.name
            );
        }
        WatchEvent::Deleted(o) => {
            info!("Deleted Pod: {}", o.metadata.name);
        }
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
