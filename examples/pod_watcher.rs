use color_eyre::Result;
use futures::prelude::*;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{ListParams, Meta},
    Api, Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());
    let api = Api::<Pod>::namespaced(client, &namespace);
    let watcher = watcher(api, ListParams::default());
    try_flatten_applied(watcher)
        .try_for_each(|p| async move {
            log::info!("Applied: {}", Meta::name(&p));
            Ok(())
        })
        .await?;
    Ok(())
}
