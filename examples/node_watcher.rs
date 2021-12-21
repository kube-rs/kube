#[macro_use] extern crate log;
use backoff::ExponentialBackoff;
use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::{Event, Node};
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{
        utils::{try_flatten_applied, StreamBackoff},
        watcher,
    },
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,node_watcher=debug,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let events: Api<Event> = Api::all(client.clone());
    let nodes: Api<Node> = Api::all(client.clone());

    let obs = try_flatten_applied(StreamBackoff::new(
        watcher(nodes, ListParams::default().labels("beta.kubernetes.io/os=linux")),
        ExponentialBackoff::default(),
    ));

    pin_mut!(obs);
    while let Some(n) = obs.try_next().await? {
        check_for_node_failures(&events, n).await?;
    }
    Ok(())
}

// A simple node problem detector
async fn check_for_node_failures(events: &Api<Event>, o: Node) -> anyhow::Result<()> {
    let name = o.name();
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
        // Turn node_watcher=debug in log to see all
        debug!("Healthy node: {}", name);
    }
    Ok(())
}
