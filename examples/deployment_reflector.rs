#[macro_use] extern crate log;
use kube::{
    api::{ResourceType, Reflector},
    client::APIClient,
    config,
};
use k8s_openapi::api::apps::v1::{DeploymentSpec, DeploymentStatus};

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let resource = ResourceType::Deploys(Some("kube-system".into()));
    let rf : Reflector<DeploymentSpec, DeploymentStatus> =
        Reflector::new(client, resource.into())
        .init()?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Deployment
    rf.read()?.into_iter().for_each(|(name, d)| {
        info!("Found deployment for {} - {} replicas running {:?}",
            name, d.status.replicas.unwrap(),
            d.spec.template.spec.unwrap().containers
                .into_iter().map(|c| c.image.unwrap()).collect::<Vec<_>>()
        );
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        rf.poll()?;
        let deploys = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        info!("Current deploys: {:?}", deploys);
    }
}
