#[macro_use] extern crate log;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ResourceExt},
    runtime::cache::{Cache, Store},
    Client,
};

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
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let cms: Api<ConfigMap> = Api::default_namespaced(client);
    let cache = Cache::new(cms, Default::default());
    let store = cache.store();

    spawn_periodic_reader(store.clone()); // read from a reader in the background

    // Observe kubernetes watch events while driving the cache:
    let mut applies = cache.applies();
    while let Some(cm) = applies.try_next().await? {
        println!("Saw cm: {} (total={})", cm.name(), store.state().len());
    }
    Ok(())
}
