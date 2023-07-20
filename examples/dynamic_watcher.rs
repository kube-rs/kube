use futures::{Stream, StreamExt, TryStreamExt};
use kube::{
    api::{Api, ApiResource, DynamicObject, GroupVersionKind, Resource, ResourceExt},
    runtime::{metadata_watcher, watcher, watcher::Event, WatchStreamExt},
};
use serde::de::DeserializeOwned;
use tracing::*;

use std::{env, fmt::Debug};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;

    // If set will receive only the metadata for watched resources
    let watch_metadata = env::var("WATCH_METADATA").map(|s| s == "1").unwrap_or(false);

    // Take dynamic resource identifiers:
    let group = env::var("GROUP").unwrap_or_else(|_| "".into());
    let version = env::var("VERSION").unwrap_or_else(|_| "v1".into());
    let kind = env::var("KIND").unwrap_or_else(|_| "Pod".into());

    // Turn them into a GVK
    let gvk = GroupVersionKind::gvk(&group, &version, &kind);
    // Use API discovery to identify more information about the type (like its plural)
    let (ar, _caps) = kube::discovery::pinned_kind(&client, &gvk).await?;

    // Use the full resource info to create an Api with the ApiResource as its DynamicType
    let api = Api::<DynamicObject>::all_with(client, &ar);
    let wc = watcher::Config::default();

    // Start a metadata or a full resource watch
    if watch_metadata {
        handle_events(metadata_watcher(api, wc), &ar).await
    } else {
        handle_events(watcher(api, wc), &ar).await
    }
}

async fn handle_events<
    K: Resource<DynamicType = ApiResource> + Clone + Debug + Send + DeserializeOwned + 'static,
>(
    stream: impl Stream<Item = watcher::Result<Event<K>>> + Send + 'static,
    ar: &ApiResource,
) -> anyhow::Result<()> {
    let mut items = stream.applied_objects().boxed();
    while let Some(p) = items.try_next().await? {
        if let Some(ns) = p.namespace() {
            info!("saw {} {} in {ns}", K::kind(ar), p.name_any());
        } else {
            info!("saw {} {}", K::kind(ar), p.name_any());
        }
        trace!("full obj: {p:?}");
    }
    Ok(())
}
