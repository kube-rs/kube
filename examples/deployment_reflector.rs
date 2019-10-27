#[macro_use] extern crate log;
use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};

#[tokio::main]
async fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Deployment(client).within(&namespace);
    let rf = Reflector::new(resource).init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> Deployment
    rf.read()?.into_iter().for_each(|deployment| {
        info!("Found deployment for {} - {} replicas running {:?}",
            deployment.metadata.name,
            deployment.status.unwrap().replicas.unwrap(),
            deployment.spec.template.spec.unwrap().containers
                .into_iter().map(|c| c.image.unwrap()).collect::<Vec<_>>()
        );
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        rf.poll().await?;
        let deploys = rf.read()?.into_iter().map(|deployment| deployment.metadata.name).collect::<Vec<_>>();
        info!("Current deploys: {:?}", deploys);
    }
}
