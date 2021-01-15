// This example shows how to use kube with dynamically known resource kinds.

use color_eyre::Result;
use futures::prelude::*;
use kube::{
    api::{DynamicObject, GroupVersionKind, ListParams, Meta},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();

    // alternatively, you can:
    // 1) take them CLI arguments
    // 2) use dynamic discovery apis on kube::Client and e.g. watch all
    // resources in cluster.
    let group = std::env::var("GROUP").expect("GROUP not set");
    let group = group.trim();
    let version = std::env::var("VERSION").expect("VERSION not set");
    let version = version.trim();
    let kind = std::env::var("KIND").expect("KIND not set");
    let kind = kind.trim();

    let gvk = GroupVersionKind::from_dynamic_gvk(group, version, kind);

    let client = Client::try_default().await?;
    let api = Api::<DynamicObject>::all_with(client, &gvk);
    let watcher = watcher(api, ListParams::default());
    try_flatten_applied(watcher)
        .try_for_each(|p| async move {
            log::info!("Applied: {}", Meta::name(&p));
            Ok(())
        })
        .await?;
    Ok(())
}
