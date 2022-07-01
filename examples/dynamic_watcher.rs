use futures::{StreamExt, TryStreamExt};
use kube::{
    api::{Api, DynamicObject, GroupVersionKind, ListParams, ResourceExt},
    discovery::{self, Scope},
    runtime::{watcher, WatchStreamExt},
    Client,
};
use tracing::*;

use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

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

    // Fully compatible with kube-runtime
    let mut items = watcher(api, ListParams::default()).applied_objects().boxed();
    while let Some(p) = items.try_next().await? {
        if caps.scope == Scope::Cluster {
            info!("saw {}", p.name_any());
        } else {
            info!("saw {} in {}", p.name_any(), p.namespace().unwrap());
        }
    }
    Ok(())
}
