#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ListParams, Meta},
    Client,
};
use kube_runtime::{reflector, reflector::Store, utils::try_flatten_applied, watcher};

fn spawn_periodic_reader(reader: Store<ConfigMap>) {
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let state = reader.state();
            let cms: Vec<_> = state.iter().map(Meta::name).collect();
            info!("Current configmaps: {:?}", cms);
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example

    let store = reflector::store::Writer::<ConfigMap>::default();
    let reader = store.as_reader();
    let rf = reflector(store, watcher(cms, lp));

    spawn_periodic_reader(reader); // read from a reader in the background

    let mut applied_events = try_flatten_applied(rf).boxed_local();
    while let Some(event) = applied_events.try_next().await? {
        info!("Applied {}", Meta::name(&event))
    }
    Ok(())
}
