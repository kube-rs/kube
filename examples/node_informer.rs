#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::{Event, Node};
use kube::{
    api::{Api, ListParams, Meta, WatchEvent},
    runtime::Informer,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,node_informer=debug,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let events: Api<Event> = Api::all(client.clone());
    let nodes: Api<Node> = Api::all(client.clone());

    let lp = ListParams::default().labels("beta.kubernetes.io/os=linux");
    let ni = Informer::new(nodes).params(lp);

    loop {
        let mut nodes = ni.poll().await?.boxed();

        while let Some(ne) = nodes.try_next().await? {
            handle_nodes(&events, ne).await?;
        }
    }
}

// This function lets the app handle an event from kube
async fn handle_nodes(events: &Api<Event>, ne: WatchEvent<Node>) -> anyhow::Result<()> {
    match ne {
        WatchEvent::Added(o) => {
            info!("New Node: {}", o.spec.unwrap().provider_id.unwrap());
        }
        WatchEvent::Modified(o) => {
            let name = Meta::name(&o);
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
                        (c.status == "True" && c.type_ != "Ready")
                            || (c.status == "False" && c.type_ == "Ready")
                    })
                    .map(|c| c.message)
                    .collect::<Vec<_>>(); // failed statuses
                warn!("Unschedulable Node: {}, ({:?})", name, failed);
                // Find events related to this node
                let opts = ListParams::default()
                    .fields(&format!("involvedObject.kind=Node,involvedObject.name={}", name));
                let evlist = events.list(&opts).await?;
                for e in evlist {
                    warn!("Node event: {:?}", serde_json::to_string_pretty(&e)?);
                }
            } else {
                // Turn up logging above to see
                debug!("Healthy node: {}", name);
            }
        }
        WatchEvent::Deleted(o) => {
            let labels = Meta::meta(&o).labels.clone().unwrap();
            warn!(
                "Deleted node: {} ({:?}) running {:?} with labels: {:?}",
                Meta::name(&o),
                o.spec.unwrap().provider_id.unwrap(),
                o.status.unwrap().conditions.unwrap(),
                labels,
            );
        }
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
        _ => {}
    }
    Ok(())
}
