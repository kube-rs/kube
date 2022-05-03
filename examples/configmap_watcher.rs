use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ListParams},
    runtime::watcher,
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    let lp = ListParams::default();

    let mut w = watcher(cms, lp).boxed();
    while let Some(event) = w.try_next().await? {
        info!("Got: {:?}", event);
    }
    Ok(())
}
