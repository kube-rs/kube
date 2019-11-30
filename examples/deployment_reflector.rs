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
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Deployment(client).within(&namespace);
    let rf = Reflector::new(resource)
        .timeout(10) // low timeout in this example
        .init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Deployment
    rf.state().await?.into_iter().for_each(|deployment| {
        info!("Found deployment for {} - {} replicas running {:?}",
            deployment.metadata.name,
            deployment.status.unwrap().replicas.unwrap(),
            deployment.spec.template.spec.unwrap().containers
                .into_iter().map(|c| c.image.unwrap()).collect::<Vec<_>>()
        );
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?;

        // Read the updated internal state (instant):
        let deploys = rf.state().await?.into_iter().map(|deployment| deployment.metadata.name).collect::<Vec<_>>();
        info!("Current deploys: {:?}", deploys);
    }
}
