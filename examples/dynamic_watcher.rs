use futures::{StreamExt, TryStreamExt};
use kube::{
    api::{Api, DynamicObject, GroupVersionKind, ResourceExt},
    runtime::{WatchStreamExt, watcher},
};
use tracing::*;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;

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

    // For metadata-only watching, use Api::<PartialObjectMeta<DynamicObject>> instead.
    // PartialObjectMeta-based Api automatically uses efficient metadata-only requests.
    let mut items = watcher(api, watcher::Config::default()).applied_objects().boxed();
    while let Some(p) = items.try_next().await? {
        if let Some(ns) = p.namespace() {
            info!("saw {kind} {} in {ns}", p.name_any());
        } else {
            info!("saw {kind} {}", p.name_any());
        }
        trace!("full obj: {p:?}");
    }
    Ok(())
}
