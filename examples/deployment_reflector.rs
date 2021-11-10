#[macro_use] extern crate log;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{Api, ResourceExt},
    runtime::cache::Reflector,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let api: Api<Deployment> = Api::default_namespaced(client);

    let (cache, store) = Reflector::new(api, Default::default());

    // We can interact with state in another thread
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            let deploys: Vec<_> = store.state().iter().map(ResourceExt::name).collect();
            info!("Current deploys: {:?}", deploys);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Drive the cache indefinitely
    cache.run().await?;
    Ok(())
}
