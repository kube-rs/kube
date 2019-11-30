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

    let resource = Api::v1Node(client);
    let rf = Reflector::new(resource)
        .labels("kubernetes.io/lifecycle=spot")
        .timeout(10) // low timeout in this example
        .init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Node
    rf.state().await?.into_iter().for_each(|object| {
        info!("Found node {} ({:?}) running {:?} with labels: {:?}",
            object.metadata.name,
            object.spec.provider_id.unwrap(),
            object.status.unwrap().conditions.unwrap(),
            object.metadata.labels,
        );
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?;

        // Read the internal state (instant):
        let deploys = rf.state().await?.into_iter().map(|o| o.metadata.name).collect::<Vec<_>>();
        info!("Current nodes: {:?}", deploys);
    }
}
