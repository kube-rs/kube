#[macro_use] extern crate log;
use kube::{
    api::{ResourceType, Informer, WatchEvent},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{
    NodeSpec, NodeStatus,
    Event, ListEventForAllNamespacesOptional,
};

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let nodes = ResourceType::Nodes;
    let ni = Informer::new(client.clone(), nodes.into())
        .labels("role=worker")
        .init()?;

    loop {
        ni.poll()?;

        while let Some(event) = ni.pop() {
            handle_nodes(&client, event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_nodes(client: &APIClient, ev: WatchEvent<NodeSpec, NodeStatus>) -> Result<(), failure::Error> {
    match ev {
        WatchEvent::Added(o) => {
            info!("New Node: {}", o.spec.provider_id.unwrap());
        },
        WatchEvent::Modified(o) => {
            // Nodes often modify a lot - only print broken nodes
            if let Some(true) = o.spec.unschedulable {
                let failed = o.status.conditions.unwrap().into_iter().filter(|c| {
                    // In a failed state either some of the extra conditions are not False
                    // Or the Ready state is False
                    (c.status == "True" && c.type_ != "Ready") ||
                    (c.status == "False" &&  c.type_ == "Ready")
                }).map(|c| c.message).collect::<Vec<_>>(); // failed statuses
                warn!("Unschedulable Node: {}, ({:?})", o.metadata.name, failed);
                // Separate API call with client to find events related to this node
                let sel = format!("involvedObject.kind=Node,involvedObject.name={}", o.metadata.name);
                let opts = ListEventForAllNamespacesOptional {
                    field_selector: Some(&sel),
                    ..Default::default()
                };
                let req = Event::list_event_for_all_namespaces(opts)?.0;
                let res = client.request::<Event>(req)?;
                warn!("Node events: {:?}", res);
            } else {
                // Turn up logging above to see
                debug!("Normal node: {}", o.metadata.name);
            }
        },
        WatchEvent::Deleted(o) => {
            warn!("Deleted node: {} ({:?}) running {:?} with labels: {:?}",
                o.metadata.name, o.spec.provider_id.unwrap(),
                o.status.conditions.unwrap(),
                o.metadata.labels,
            );
        },
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
