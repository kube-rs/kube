#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf = Reflector::new(pods).params(lp);
    let runner = rf.clone().run();

    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let pods: Vec<_> = rf.state().await.unwrap().iter().map(Meta::name).collect();
            info!("Current pods: {:?}", pods);
        }
    });
    runner.await?;
    Ok(())
}
