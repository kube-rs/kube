#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, ListParams, Meta},
    Client,
};
use kube_runtime::{reflector, utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let store = reflector::store::Writer::<Deployment>::default();
    let reader = store.as_reader();
    let rf = reflector(
        store,
        watcher(
            Api::<Deployment>::namespaced(client.clone(), &namespace),
            ListParams::default().timeout(10), // short watch timeout in this example
        ),
    );

    // We can interact with state in another thread
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            let deploys: Vec<_> = reader.state().iter().map(|o| Meta::name(o).to_string()).collect();
            info!("Current deploys: {:?}", deploys);
            tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
        }
    });

    // We can look at the events we want and use it as a watcher
    let mut rfa = try_flatten_applied(rf).boxed();
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {}", Meta::name(&event));
    }

    Ok(())
}
