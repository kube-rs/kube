#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{ListParams, Resource},
    client::APIClient,
    config,
    runtime::Reflector,
};

/// Example way to read secrets
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Resource::namespaced::<ConfigMap>(&namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf: Reflector<ConfigMap> = Reflector::new(client, lp, resource).init().await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|cm| {
        info!(
            "Found configmap {} with data: {:?}",
            cm.metadata.unwrap().name.unwrap(),
            cm.data
        );
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?; // ideally call this from a thread/task

        // up to date state:
        let pods = rf
            .state()
            .await?
            .into_iter()
            .map(|cm| cm.metadata.unwrap().name.unwrap())
            .collect::<Vec<_>>();

        info!("Current configmaps: {:?}", pods);
    }
}
