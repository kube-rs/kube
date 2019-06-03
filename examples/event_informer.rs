#[macro_use] extern crate log;
use kube::{
    api::{Api, Informer, WatchEvent},
    api::Event,
    client::APIClient,
    config,
};


fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let events = Api::v1Event(client);
    let ei = Informer::new(events)
        .init()?;

    loop {
        ei.poll()?;

        while let Some(event) = ei.pop() {
            handle_events(event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_events(ev: WatchEvent<Event>) -> Result<(), failure::Error> {
    match ev {
        WatchEvent::Added(o) => {
            info!("New Event: {}, {}", o.type_, o.message);
        },
        WatchEvent::Modified(o) => {
            info!("Modified Event: {}", o.reason);
        },
        WatchEvent::Deleted(o) => {
            info!("Deleted Event: {}", o.message);
        },
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
