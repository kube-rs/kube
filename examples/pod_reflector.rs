use futures::prelude::*;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams},
    runtime::{reflector, watcher},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let api: Api<Pod> = Api::namespaced(client, &namespace);
    let store_w = reflector::store::Writer::default();
    let store = store_w.as_reader();
    let reflector = reflector(store_w, watcher(api, ListParams::default()));
    // Use try_for_each to fail on first error, use for_each to keep retrying
    reflector
        .try_for_each(|_event| async {
            info!("Current pod count: {}", store.state().len());
            Ok(())
        })
        .await?;
    Ok(())
}
