extern crate failure;
extern crate k8s_openapi;
extern crate kube;

use kube::{
    api::{ResourceType, Reflector, WatchEvents, WatchEvent},
    client::APIClient,
    config,
};

use k8s_openapi::api::core::v1::{PodSpec, PodStatus};

fn main() -> Result<(), failure::Error> {
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = ResourceType::Pods(Some("kube-system".into()));
    let rf : Reflector<PodSpec, PodStatus> = Reflector::new(client.clone(), resource.into())?;

    rf.read()?.into_iter().for_each(|(name, p)| {
        println!("Found pod {} ({}) with {:?}",
            name,
            p.status.phase.unwrap(),
            p.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>(),
        );
    });

    // Here we both poll and reconcile based on events from the main thread
    // If you run this next to actix-web (say), spawn a thread and pass `rf` as app state
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        rf.poll()?;
        let pods = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        println!("Current pods: {:?}", pods);

        // After polling, handle the events:
        let events = rf.events()?;
        reconcile(&client, events)?;
    }
}

// This function lets the app handle an events from kube watch as they occur
// Once this function has been completed, the events are gone from the reflector's state.
fn reconcile(_client: &APIClient, events: WatchEvents<PodSpec, PodStatus>) -> Result<(), failure::Error> {
    for ev in &events {
        println!("Got {:?}", ev);
        match ev {
            // TODO: do other kube api calls with client here...
            WatchEvent::Added(o) => {
                println!("Handling Added in {}", o.metadata.name);
            },
            WatchEvent::Modified(o) => {
                println!("Handling Modified Pod in {}", o.metadata.name);
            },
            WatchEvent::Deleted(o) => {
                println!("Handling Deleted Pod in {}", o.metadata.name);
            },
            WatchEvent::Error(e) => {
                println!("Error event: {:?}", e); // ought to refresh here
            }
        }
    }
    Ok(())
}
