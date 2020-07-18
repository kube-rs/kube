#[macro_use] extern crate log;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Event;
use kube::{
    api::{Api, ListParams},
    Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let events: Api<Event> = Api::all(client);
    let lp = ListParams::default();
    let mut ew = Box::pin(try_flatten_applied(watcher(events, lp)));

    while let Some(event) = ew.try_next().await? {
        handle_event(event)?;
    }
    Ok(())
}

// This function lets the app handle an added/modified event from k8s
fn handle_event(ev: Event) -> anyhow::Result<()> {
    info!(
        "New Event: {} (via {} {})",
        ev.message.unwrap(),
        ev.involved_object.kind.unwrap(),
        ev.involved_object.name.unwrap()
    );
    Ok(())
}
