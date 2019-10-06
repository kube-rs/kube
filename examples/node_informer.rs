#[macro_use] extern crate log;
use kube::{
    api::{RawApi, Api, v1Event, Informer, ListParams, WatchEvent, Object},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};

type Node = Object<NodeSpec, NodeStatus>;
type Event = v1Event; // snowflake obj

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,node_informer=debug,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let nodes = RawApi::v1Node();
    let events = Api::v1Event(client.clone());
    let ni = Informer::raw(client.clone(), nodes)
        .labels("role=worker")
        .init()?;

    loop {
        ni.poll()?;

        while let Some(ne) = ni.pop() {
            handle_nodes(&events, ne)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_nodes(events: &Api<Event>, ne: WatchEvent<Node>) -> Result<(), failure::Error> {
    match ne {
        WatchEvent::Added(o) => {
            info!("New Node: {}", o.spec.provider_id.unwrap());
        },
        WatchEvent::Modified(o) => {
            // Nodes often modify a lot - only print broken nodes
            if let Some(true) = o.spec.unschedulable {
                let failed = o.status.unwrap().conditions.unwrap().into_iter().filter(|c| {
                    // In a failed state either some of the extra conditions are not False
                    // Or the Ready state is False
                    (c.status == "True" && c.type_ != "Ready") ||
                    (c.status == "False" &&  c.type_ == "Ready")
                }).map(|c| c.message).collect::<Vec<_>>(); // failed statuses
                warn!("Unschedulable Node: {}, ({:?})", o.metadata.name, failed);
                // Find events related to this node
                let sel = format!("involvedObject.kind=Node,involvedObject.name={}", o.metadata.name);
                let opts = ListParams {
                    field_selector: Some(sel),
                    ..Default::default()
                };
                let evlist = events.list(&opts)?;
                for e in evlist.items {
                    warn!("Node event: {:?}", serde_json::to_string_pretty(&e)?);
                }
            } else {
                // Turn up logging above to see
                debug!("Normal node: {}", o.metadata.name);
            }
        },
        WatchEvent::Deleted(o) => {
            warn!("Deleted node: {} ({:?}) running {:?} with labels: {:?}",
                o.metadata.name, o.spec.provider_id.unwrap(),
                o.status.unwrap().conditions.unwrap(),
                o.metadata.labels,
            );
        },
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
