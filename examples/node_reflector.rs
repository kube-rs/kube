use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{reflector, watcher, WatchStreamExt},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default()
        .labels("kubernetes.io/arch=amd64") // filter instances by label
        .timeout(10); // short watch timeout in this example

    let (reader, writer) = reflector::store();
    let rf = reflector(writer, watcher(nodes, lp));

    // Periodically read our state in the background
    tokio::spawn(async move {
        loop {
            let nodes = reader.state().iter().map(|r| r.name_any()).collect::<Vec<_>>();
            info!("Current {} nodes: {:?}", nodes.len(), nodes);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Drain and log applied events from the reflector
    let mut rfa = rf.applied_objects().boxed();
    while let Some(event) = rfa.try_next().await? {
        info!("saw {}", event.name_any());
    }

    Ok(())
}
