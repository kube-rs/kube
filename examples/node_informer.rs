#[macro_use] extern crate log;
use futures::StreamExt;
use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};
use kube::{
    api::{v1Event, Api, Informer, ListParams, Object, RawApi, WatchEvent},
    client::APIClient,
    config,
};

type Node = Object<NodeSpec, NodeStatus>;
type Event = v1Event; // snowflake obj

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,node_informer=debug,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    let nodes = RawApi::v1Node();
    let events = Api::v1Event(client.clone());
    let ni = Informer::raw(client.clone(), nodes)
        //.labels("beta.kubernetes.io/os=linux")
        .init()
        .await?;

    loop {
        let mut nodes = ni.poll().await?.boxed();

        while let Some(ne) = nodes.next().await {
            let ne = ne?;
            handle_nodes(&events, ne).await?;
        }
    }
}

// This function lets the app handle an event from kube
async fn handle_nodes(events: &Api<Event>, ne: WatchEvent<Node>) -> anyhow::Result<()> {
    match ne {
        WatchEvent::Added(o) => {
            info!("New Node: {}", o.spec.provider_id.unwrap());
        }
        WatchEvent::Modified(o) => {
            // Nodes often modify a lot - only print broken nodes
            if let Some(true) = o.spec.unschedulable {
                let failed = o
                    .status
                    .unwrap()
                    .conditions
                    .unwrap()
                    .into_iter()
                    .filter(|c| {
                        // In a failed state either some of the extra conditions are not False
                        // Or the Ready state is False
                        (c.status == "True" && c.type_ != "Ready")
                            || (c.status == "False" && c.type_ == "Ready")
                    })
                    .map(|c| c.message)
                    .collect::<Vec<_>>(); // failed statuses
                warn!("Unschedulable Node: {}, ({:?})", o.metadata.name, failed);
                // Find events related to this node
                let sel = format!("involvedObject.kind=Node,involvedObject.name={}", o.metadata.name);
                let opts = ListParams {
                    field_selector: Some(sel),
                    ..Default::default()
                };
                let evlist = events.list(&opts).await?;
                for e in evlist {
                    warn!("Node event: {:?}", serde_json::to_string_pretty(&e)?);
                }
            } else {
                // Turn up logging above to see
                debug!("Normal node: {}", o.metadata.name);
            }
        }
        WatchEvent::Deleted(o) => {
            warn!(
                "Deleted node: {} ({:?}) running {:?} with labels: {:?}",
                o.metadata.name,
                o.spec.provider_id.unwrap(),
                o.status.unwrap().conditions.unwrap(),
                o.metadata.labels,
            );
        }
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
