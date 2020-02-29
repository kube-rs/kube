#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{RawApi, WatchEvent},
    client::APIClient,
    config,
    runtime::Informer,
};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = env::var("NAMESPACE").unwrap_or("default".into());

    let resource = RawApi::namespaced::<Pod>(&namespace);
    let inf = Informer::raw(client.clone(), resource.clone());

    loop {
        let mut pods = inf.poll().await?.boxed();

        while let Some(event) = pods.try_next().await? {
            handle_node(&resource, event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_node(_pods: &RawApi, ev: WatchEvent<Pod>) -> anyhow::Result<()> {
    match ev {
        WatchEvent::Added(o) => {
            let containers = o
                .spec
                .unwrap()
                .containers
                .into_iter()
                .map(|c| c.name)
                .collect::<Vec<_>>();
            info!(
                "Added Pod: {} (containers={:?})",
                o.metadata.unwrap().name.unwrap(),
                containers
            );
        }
        WatchEvent::Modified(o) => {
            let meta = o.metadata.unwrap();
            let phase = o.status.unwrap().phase.unwrap();
            let owner = &meta.owner_references.unwrap()[0];
            info!(
                "Modified Pod: {} (phase={}, owner={})",
                meta.name.unwrap(),
                phase,
                owner.name
            );
        }
        WatchEvent::Deleted(o) => {
            info!("Deleted Pod: {}", o.metadata.unwrap().name.unwrap());
        }
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
