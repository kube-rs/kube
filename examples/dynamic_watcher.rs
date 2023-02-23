use futures::{Stream, StreamExt, TryStreamExt};
use kube::{
    api::{Api, DynamicObject, GroupVersionKind, ListParams, ResourceExt},
    discovery::{self, ApiCapabilities, Scope},
    runtime::{metadata_watcher, watcher, WatchStreamExt},
    Client,
};
use serde::de::DeserializeOwned;
use tracing::*;

use std::{env, fmt::Debug};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // If set will receive only the metadata for watched resources
    let watch_metadata = env::var("WATCH_METADATA").map(|s| s == "1").unwrap_or(false);

    // Take dynamic resource identifiers:
    let group = env::var("GROUP").unwrap_or_else(|_| "clux.dev".into());
    let version = env::var("VERSION").unwrap_or_else(|_| "v1".into());
    let kind = env::var("KIND").unwrap_or_else(|_| "Foo".into());

    // Turn them into a GVK
    let gvk = GroupVersionKind::gvk(&group, &version, &kind);
    // Use API discovery to identify more information about the type (like its plural)
    let (ar, caps) = discovery::pinned_kind(&client, &gvk).await?;

    // Use the full resource info to create an Api with the ApiResource as its DynamicType
    let api = Api::<DynamicObject>::all_with(client, &ar);

    // Start a metadata or a full resource watch
    if watch_metadata {
        handle_events(metadata_watcher(api, ListParams::default()), caps).await?
    } else {
        handle_events(watcher(api, ListParams::default()), caps).await?
    }

    Ok(())
}

async fn handle_events<K: kube::Resource + Clone + Debug + Send + DeserializeOwned + 'static>(
    stream: impl Stream<Item = watcher::Result<watcher::Event<K>>> + Send + 'static,
    api_caps: ApiCapabilities,
) -> anyhow::Result<()> {
    // Fully compatible with kube-runtime
    let mut items = stream.applied_objects().boxed();
    while let Some(p) = items.try_next().await? {
        if api_caps.scope == Scope::Cluster {
            info!("saw {}", p.name_any());
        } else {
            info!("saw {} in {}", p.name_any(), p.namespace().unwrap());
        }
    }

    Ok(())
}
