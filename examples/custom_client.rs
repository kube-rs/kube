// Minimal custom client example.
use k8s_openapi::api::core::v1::ConfigMap;
use tower::ServiceBuilder;

use kube::{
    api::{Api, ListParams},
    client::{ConfigExt, SetBaseUriLayer},
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;
    let https = config.native_tls_https_connector()?;
    let client = Client::new(
        ServiceBuilder::new()
            .layer(SetBaseUriLayer::new(config.cluster_url))
            .service(hyper::Client::builder().build(https)),
    );

    let cms: Api<ConfigMap> = Api::namespaced(client, "default");
    for cm in cms.list(&ListParams::default()).await? {
        println!("{:?}", cm);
    }

    Ok(())
}
