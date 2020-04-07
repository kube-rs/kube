#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
};

/// Example way to read secrets
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf = Reflector::new(cms, lp).init().await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|cm| {
        info!("Found configmap {} with data: {:?}", Meta::name(&cm), cm.data);
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?; // ideally call this from a thread/task

        // up to date state:
        let pods: Vec<_> = rf.state().await?.iter().map(Meta::name).collect();
        info!("Current configmaps: {:?}", pods);
    }
}
