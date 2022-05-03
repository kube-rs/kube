use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{reflector, utils::try_flatten_applied, watcher},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

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
            let deploys: Vec<_> = reader.state().iter().map(|r| r.name()).collect();
            info!("Current deploys: {:?}", deploys);
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        }
    });

    // We can look at the events we want and use it as a watcher
    let mut rfa = try_flatten_applied(rf).boxed();
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {}", event.name());
    }

    Ok(())
}
