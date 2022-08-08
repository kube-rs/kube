use backoff::ExponentialBackoff;
use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::{Event, Node};
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{watcher, WatchStreamExt},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let events: Api<Event> = Api::all(client.clone());
    let nodes: Api<Node> = Api::all(client.clone());

    let lp = ListParams::default().labels("beta.kubernetes.io/arch=amd64");
    let obs = watcher(nodes, lp)
        .backoff(ExponentialBackoff::default())
        .applied_objects();

    pin_mut!(obs);
    while let Some(n) = obs.try_next().await? {
        check_for_node_failures(&events, n).await?;
    }
    Ok(())
}

// A simple node problem detector
async fn check_for_node_failures(events: &Api<Event>, o: Node) -> anyhow::Result<()> {
    let name = o.name_any();
    // Nodes often modify a lot - only print broken nodes
    if let Some(true) = o.spec.unwrap().unschedulable {
        let failed = o
            .status
            .unwrap()
            .conditions
            .unwrap()
            .into_iter()
            .filter(|c| {
                // In a failed state either some of the extra conditions are not False
                // Or the Ready state is False
                (c.status == "True" && c.type_ != "Ready") || (c.status == "False" && c.type_ == "Ready")
            })
            .map(|c| c.message)
            .collect::<Vec<_>>(); // failed statuses
        warn!("Unschedulable Node: {}, ({:?})", name, failed);
        // Find events related to this node
        let opts =
            ListParams::default().fields(&format!("involvedObject.kind=Node,involvedObject.name={}", name));
        let evlist = events.list(&opts).await?;
        for e in evlist {
            warn!("Node event: {:?}", serde_json::to_string_pretty(&e)?);
        }
    } else {
        info!("Healthy node: {}", name);
    }
    Ok(())
}
