#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Node;
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

    let nodes: Api<Node> = Api::all(client.clone());
    let lp = ListParams::default()
        .labels("beta.kubernetes.io/instance-type=m4.2xlarge") // filter instances by label
        .timeout(10); // short watch timeout in this example
    let rf = Reflector::new(nodes).params(lp);

    let rf2 = rf.clone(); // read from a clone in a task
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let deploys: Vec<_> = rf2.state().await.unwrap().iter().map(Meta::name).collect();
            info!("Current {} nodes: {:?}", deploys.len(), deploys);
        }
    });
    rf.run().await?; // run reflector and listen for signals
    Ok(())
}
