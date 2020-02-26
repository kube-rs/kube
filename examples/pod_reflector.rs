#[macro_use] extern crate log;
use futures_timer::Delay;
use kube::{api::Api, client::APIClient, config, runtime::Reflector};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Pod(client).within(&namespace);
    let rf = Reflector::new(resource)
        .timeout(20) // low timeout in this example
        .init()
        .await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|pod| {
        info!(
            "Found initial pod {} ({}) with {:?}",
            pod.metadata.name,
            pod.status.unwrap().phase.unwrap(),
            pod.spec
                .containers
                .into_iter()
                .map(|c| c.name)
                .collect::<Vec<_>>(),
        );
    });

    let cloned = rf.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = cloned.poll().await {
                warn!("Poll error: {:?}", e);
            }
        }
    });


    loop {
        Delay::new(Duration::from_secs(5)).await;
        let pods = rf
            .state()
            .await?
            .into_iter()
            .map(|pod| pod.metadata.name)
            .collect::<Vec<_>>();
        info!("Current pods: {:?}", pods);
    }
}
