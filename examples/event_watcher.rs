use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::core::v1::Event;
use kube::{
    api::{Api, ListParams},
    runtime::{watcher, WatchStreamExt},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let events: Api<Event> = Api::all(client);
    let lp = ListParams::default();

    let ew = watcher(events, lp).applied_objects();

    pin_mut!(ew);
    while let Some(event) = ew.try_next().await? {
        handle_event(event)?;
    }
    Ok(())
}

// This function lets the app handle an added/modified event from k8s
fn handle_event(ev: Event) -> anyhow::Result<()> {
    info!(
        "Event: \"{}\" via {} {}",
        ev.message.unwrap().trim(),
        ev.involved_object.kind.unwrap(),
        ev.involved_object.name.unwrap()
    );
    Ok(())
}
