use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{predicates, reflector, watcher, WatchStreamExt},
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

    let (reader, writer) = reflector::store::<Node>();

    // Periodically read our state in the background
    tokio::spawn(async move {
        loop {
            let nodes = reader.state().iter().map(|r| r.name()).collect::<Vec<_>>();
            info!("Current {} nodes: {:?}", nodes.len(), nodes);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Drain and log applied events from the reflector
    let rf = reflector(writer, watcher(nodes, lp))
        .applied_objects()
        .predicate_filter(predicates::labels);
    pin_mut!(rf);
    while let Some(node) = rf.try_next().await? {
        info!("saw node {} with changed labels", node.name());
    }

    Ok(())
}
