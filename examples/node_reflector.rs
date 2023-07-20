use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ResourceExt},
    runtime::{predicates, reflector, watcher, Predicate, WatchStreamExt},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let nodes: Api<Node> = Api::all(client.clone());
    let wc = watcher::Config::default()
        .labels("kubernetes.io/arch=amd64") // filter instances by label
        .timeout(10); // short watch timeout in this example

    let (reader, writer) = reflector::store();
    let stream = watcher(nodes, wc)
        .default_backoff()
        .reflect(writer)
        .applied_objects()
        .predicate_filter(predicates::labels.combine(predicates::annotations)); // NB: requires an unstable feature

    // Periodically read our state in the background
    tokio::spawn(async move {
        reader.wait_until_ready().await.unwrap();
        loop {
            let nodes = reader.state().iter().map(|r| r.name_any()).collect::<Vec<_>>();
            info!("Current {} nodes: {:?}", nodes.len(), nodes);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Log applied events with changes from the reflector
    pin_mut!(stream);
    while let Some(node) = stream.try_next().await? {
        info!("saw node {} with new labels/annots", node.name_any());
    }

    Ok(())
}
