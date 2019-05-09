#[macro_use] extern crate log;
use kube::{
    api::{ResourceType, Informer, WatchEvents, WatchEvent},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{PodSpec, PodStatus};

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = ResourceType::Pods(Some("kube-system".into()));
    let inf : Informer<PodSpec, PodStatus> = Informer::new(client.clone(), resource.into())?;

    // Here we both poll and reconcile based on events from the main thread
    // If you run this next to actix-web (say), spawn a thread and pass `inf` as app state
    loop {
        let events = inf.poll()?;

        // After polling, handle the events
        reconcile(&client, events)?;

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}

// This function lets the app handle an events from kube watch as they occur
// Once this function has been completed, the events are gone from the reflector's state.
fn reconcile(_c: &APIClient, events: WatchEvents<PodSpec, PodStatus>) -> Result<(), failure::Error> {
    for ev in events {
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
    }
    Ok(())
}
