#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Node;
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

    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default()
        .labels("beta.kubernetes.io/instance-type=m4.2xlarge") // filter instances by label
        .timeout(10); // short watch timeout in this example

    let store = reflector::store::Writer::<Node>::default();
    let reader = store.as_reader();
    let rf = reflector(store, watcher(nodes, lp));

    // Periodically read our state in the background
    tokio::spawn(async move {
        loop {
            let nodes = reader
                .state()
                .iter()
                .map(|o| Meta::name(o).to_string())
                .collect::<Vec<_>>();
            info!("Current {} nodes: {:?}", nodes.len(), nodes);
            tokio::time::delay_for(std::time::Duration::from_secs(10)).await;
        }
    });

    // Drain and log applied events from the reflector
    let mut rfa = try_flatten_applied(rf).boxed();
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {}", Meta::name(&event));
    }

    Ok(())
}
