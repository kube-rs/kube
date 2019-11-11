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
        .init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Node
    rf.read()?.into_iter().for_each(|object| {
        info!("Found node {} ({:?}) running {:?} with labels: {:?}",
            object.metadata.name,
            object.spec.provider_id.unwrap(),
            object.status.unwrap().conditions.unwrap(),
            object.metadata.labels,
        );
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        rf.poll().await?;
        let deploys = rf.read()?.into_iter().map(|object| object.metadata.name).collect::<Vec<_>>();
        info!("Current nodes: {:?}", deploys);
    }
}
