use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::{Event, Node};
use kube::{
    api::{Api, ListParams, ResourceExt},
    client::{scope, Client},
    runtime::{watcher, WatchStreamExt},
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let nodes: Api<Node> = Api::all(client.clone());

    let use_watchlist = std::env::var("WATCHLIST").map(|s| s == "1").unwrap_or(false);
    let wc = if use_watchlist {
        // requires WatchList feature gate on 1.27 or later
        watcher::Config::default().streaming_lists()
    } else {
        watcher::Config::default()
    };
    let obs = watcher(nodes, wc).default_backoff().applied_objects();

    pin_mut!(obs);
    while let Some(n) = obs.try_next().await? {
        check_for_node_failures(&client, n).await?;
    }
    Ok(())
}

// A simple node problem detector
async fn check_for_node_failures(client: &Client, o: Node) -> anyhow::Result<()> {
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
            ListParams::default().fields(&format!("involvedObject.kind=Node,involvedObject.name={name}"));
        let evlist = client.list::<Event>(&opts, &scope::Cluster).await?;
        for e in evlist {
            warn!("Node event: {:?}", serde_json::to_string_pretty(&e)?);
        }
    } else {
        info!("Healthy node: {}", name);
    }
    Ok(())
}
