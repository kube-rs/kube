#[macro_use] extern crate log;
use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};

/// Example way to read secrets
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1ConfigMap(client).within(&namespace);
    let rf = Reflector::new(resource)
        .timeout(10) // low timeout in this example
        .init()
        .await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|cm| {
        info!("Found configmap {} with data: {:?}", cm.metadata.name, cm.data);
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?; // ideally call this from a thread/task

        // up to date state:
        let pods = rf
            .state()
            .await?
            .into_iter()
            .map(|cm| cm.metadata.name)
            .collect::<Vec<_>>();

        info!("Current configmaps: {:?}", pods);
    }
}
