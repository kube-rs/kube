use futures::prelude::*;
use kube::{
    api::{ApiResource, DynamicObject, GroupVersionKind, ListParams, ResourceExt},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    // Take dynamic resource identifiers:
    let group = env::var("GROUP").unwrap_or_else(|_| "clux.dev".into());
    let version = env::var("VERSION").unwrap_or_else(|_| "v1".into());
    let kind = env::var("KIND").unwrap_or_else(|_| "Foo".into());

    // Turn them into a GVK
    let gvk = GroupVersionKind::gvk(&group, &version, &kind);
    let mut api_resource = ApiResource::from_gvk(&gvk);

    if let Some(resource) = env::var("RESOURCE").ok() {
        api_resource.plural = resource;
    } else {
        println!(
            "Using inferred plural name (use RESOURCE to override): {}",
            api_resource.plural
        );
    }

    // Use them in an Api with the GVK as its DynamicType
    let api = Api::<DynamicObject>::all_with(client, &api_resource);

    // Fully compatible with kube-runtime
    let watcher = watcher(api, ListParams::default());
    try_flatten_applied(watcher)
        .try_for_each(|p| async move {
            log::info!("Applied: {}", p.name());
            Ok(())
        })
        .await?;
    Ok(())
}
