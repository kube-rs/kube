#[macro_use] extern crate log;
use kube::{
    api::{v1Event, Api, WatchEvent},
    client::APIClient,
    config,
    runtime::Informer,
};

use futures::StreamExt;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    let events = Api::v1Event(client);
    let ei = Informer::new(events);

    loop {
        let mut events = ei.poll().await?.boxed();

        while let Some(event) = events.next().await {
            let event = event?;
            handle_events(event)?;
        }
    }
}

// This function lets the app handle an event from kube
fn handle_events(ev: WatchEvent<v1Event>) -> anyhow::Result<()> {
    match ev {
        WatchEvent::Added(o) => {
            info!("New Event: {}, {}", o.type_, o.message);
        }
        WatchEvent::Modified(o) => {
            info!("Modified Event: {}", o.reason);
        }
        WatchEvent::Deleted(o) => {
            info!("Deleted Event: {}", o.message);
        }
        WatchEvent::Error(e) => {
            warn!("Error event: {:?}", e);
        }
    }
    Ok(())
}
