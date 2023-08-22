use futures::{pin_mut, TryStreamExt};
use k8s_openapi::api::{core::v1::ObjectReference, events::v1::Event};
use kube::{
    api::Api,
    runtime::{watcher, WatchStreamExt},
    Client,
};
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let events: Api<Event> = Api::all(client);
    let ew = watcher(events, watcher::Config::default()).applied_objects();

    pin_mut!(ew);
    while let Some(event) = ew.try_next().await? {
        handle_event(event)?;
    }
    Ok(())
}

// This function lets the app handle an added/modified event from k8s
fn handle_event(ev: Event) -> anyhow::Result<()> {
    info!(
        "{}: {} ({})",
        ev.regarding.map(fmt_obj_ref).unwrap_or_default(),
        ev.reason.unwrap_or_default(),
        ev.note.unwrap_or_default(),
    );
    Ok(())
}

fn fmt_obj_ref(oref: ObjectReference) -> String {
    format!(
        "{}/{}",
        oref.kind.unwrap_or_default(),
        oref.name.unwrap_or_default()
    )
}
