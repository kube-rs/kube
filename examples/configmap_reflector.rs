use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{reflector, reflector::Store, utils::try_flatten_applied, watcher},
    Client,
};
use tracing::*;

fn spawn_periodic_reader(reader: Store<ConfigMap>) {
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let cms: Vec<_> = reader.state().iter().map(|obj| obj.name()).collect();
            info!("Current configmaps: {:?}", cms);
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example

    let store = reflector::store::Writer::<ConfigMap>::default();
    let reader = store.as_reader();
    let rf = reflector(store, watcher(cms, lp));

    spawn_periodic_reader(reader); // read from a reader in the background

    let mut applied_events = try_flatten_applied(rf).boxed_local();
    while let Some(event) = applied_events.try_next().await? {
        info!("Applied {}", event.name())
    }
    Ok(())
}
