#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
};
use std::time::Duration;
use tokio::time::delay_for;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf = Reflector::new(pods, lp).init().await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|pod| {
        let name = Meta::name(&pod);
        let phase = pod.status.unwrap().phase.unwrap();
        let containers = pod
            .spec
            .unwrap()
            .containers
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>();
        info!("Found initial pod {} ({}) with {:?}", name, phase, containers);
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
        delay_for(Duration::from_secs(5)).await;
        let pods: Vec<_> = rf.state().await?.iter().map(Meta::name).collect();
        info!("Current pods: {:?}", pods);
    }
}
