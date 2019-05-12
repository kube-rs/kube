#[macro_use] extern crate log;
use std::env;
use kube::{
    api::{ResourceType, Informer, WatchEvent},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};

fn main() -> Result<(), failure::Error> {
    env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);
    let namespace = Some(env::var("NAMESPACE").unwrap_or("kube-system".into()));

    let resource = ResourceType::Pods(namespace);
    let inf : Informer<PodSpec, PodStatus> = Informer::new(client.clone(), resource.into())?;

    // Here we both poll and reconcile based on events from the main thread
    // If you run this next to actix-web (say), spawn a thread and pass `inf` as app state
    loop {
        inf.poll()?;

        // Handle events one by one, draining the informer
        while let Some(event) = inf.pop() {
            reconcile(&client, event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn reconcile(_c: &APIClient, ev: WatchEvent<PodSpec, PodStatus>) -> Result<(), failure::Error> {
    // TODO: Use the kube api client here..
    match ev {
        WatchEvent::Added(o) => {
            let containers = o.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>();
            info!("Added Pod: {} (containers={:?})", o.metadata.name, containers);
        },
        WatchEvent::Modified(o) => {
            let phase = o.status.phase.unwrap();
            info!("Modified Pod: {} (phase={})", o.metadata.name, phase);
        },
        WatchEvent::Deleted(o) => {
            info!("Deleted Pod: {}", o.metadata.name);
        },
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e); // ought to refresh here
        }
    }
    Ok(())
}
