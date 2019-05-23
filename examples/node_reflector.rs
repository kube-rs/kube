#[macro_use] extern crate log;
use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};
use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = Api::v1Node();
    let rf : Reflector<NodeSpec, NodeStatus> = Reflector::new(client, resource.into())
        .labels("role=master")
        .init()?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Node
    rf.read()?.into_iter().for_each(|(name, n)| {
        info!("Found node {} ({:?}) running {:?} with labels: {:?}",
            name, n.spec.provider_id.unwrap(),
            n.status.conditions.unwrap(),
            n.metadata.labels,
        );
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        rf.poll()?;
        let deploys = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        info!("Current nodes: {:?}", deploys);
    }
}
