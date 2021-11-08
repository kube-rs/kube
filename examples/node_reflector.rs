#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Node;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::cache::Cache,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default().labels("kubernetes.io/arch=amd64");

    let cache = Cache::new(nodes, lp);
    let store = cache.store();

    // Periodically read our state in the background
    tokio::spawn(async move {
        loop {
            let nodes = store.state().iter().map(ResourceExt::name).collect::<Vec<_>>();
            info!("Current {} nodes: {:?}", nodes.len(), nodes);
            tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        }
    });

    // Run the reflector and discard informational watch events
    cache.run().await?;
    Ok(())
}
