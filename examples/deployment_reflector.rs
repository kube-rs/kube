#[macro_use] extern crate log;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{ListParams, Resource},
    client::APIClient,
    config,
    runtime::Reflector,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Resource::namespaced::<Deployment>(&namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf: Reflector<Deployment> = Reflector::new(client, lp, resource).init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Deployment
    rf.state().await?.into_iter().for_each(|d| {
        info!(
            "Found deployment for {} - {} replicas running {:?}",
            d.metadata.unwrap().name.unwrap(),
            d.status.unwrap().replicas.unwrap(),
            d.spec
                .unwrap()
                .template
                .spec
                .unwrap()
                .containers
                .into_iter()
                .map(|c| c.image.unwrap())
                .collect::<Vec<_>>()
        );
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?;

        // Read the updated internal state (instant):
        let deploys = rf
            .state()
            .await?
            .into_iter()
            .map(|deployment| deployment.metadata.unwrap().name.unwrap())
            .collect::<Vec<_>>();
        info!("Current deploys: {:?}", deploys);
    }
}
