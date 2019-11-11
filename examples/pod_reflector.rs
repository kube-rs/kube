#[macro_use] extern crate log;
use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Pod(client).within(&namespace);
    let rf = Reflector::new(resource).init().await?;

    // Can read initial state now:
    rf.read()?.into_iter().for_each(|pod| {
        info!("Found pod {} ({}) with {:?}",
            pod.metadata.name,
            pod.status.unwrap().phase.unwrap(),
            pod.spec.containers.into_iter().map(|c| c.name).collect::<Vec<_>>(),
        );
    });

    // Poll to keep data up to date:
    loop {
        rf.poll().await?;

        // up to date state:
        let pods = rf.read()?.into_iter().map(|pod| pod.metadata.name).collect::<Vec<_>>();
        info!("Current pods: {:?}", pods);
    }
}
