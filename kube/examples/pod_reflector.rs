#[macro_use] extern crate log;
use futures_timer::Delay;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{ListParams, Meta, Resource},
    runtime::Reflector,
    Client, Configuration,
};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::from(Configuration::inferred().await?);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Resource::namespaced::<Pod>(&namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf: Reflector<Pod> = Reflector::new(client, lp, resource).init().await?;

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
        Delay::new(Duration::from_secs(5)).await;
        let pods: Vec<_> = rf.state().await?.iter().map(Meta::name).collect();
        info!("Current pods: {:?}", pods);
    }
}
