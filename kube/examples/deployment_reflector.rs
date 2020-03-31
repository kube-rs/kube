#[macro_use]
extern crate log;
use k8s_openapi::api::apps::v1::Deployment;
use kube::{
    api::{ListParams, Meta, Resource},
    runtime::Reflector,
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::default().await?;

    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Resource::namespaced::<Deployment>(&namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf: Reflector<Deployment> = Reflector::new(client, lp, resource).init().await?;

    // rf is initialized with full state, which can be extracted on demand.
    // Output is an owned Vec<Deployment>
    rf.state().await?.into_iter().for_each(|d| {
        info!(
            "Found deployment for {} - {} replicas running {:?}",
            Meta::name(&d),
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
        let deploys: Vec<_> = rf.state().await?.iter().map(Meta::name).collect();
        info!("Current deploys: {:?}", deploys);
    }
}
