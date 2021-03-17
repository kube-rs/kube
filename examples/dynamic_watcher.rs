use futures::prelude::*;
use kube::{
    api::{DynamicObject, GroupVersionKind, ListParams, Meta},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    // Take dynamic resource identifiers:
    let group = "clux.dev";
    let version = "v1";
    let kind = "Foo";

    // Turn them into a GVK
    let gvk = GroupVersionKind::from_dynamic_gvk(group, version, kind);
    // Use them in an Api with the dynamic family
    let api = Api::<DynamicObject>::all_with(client, &gvk);

    // Fully compatible with kube-runtime
    let watcher = watcher(api, ListParams::default());
    try_flatten_applied(watcher)
        .try_for_each(|p| async move {
            log::info!("Applied: {}", Meta::name(&p));
            Ok(())
        })
        .await?;
    Ok(())
}
