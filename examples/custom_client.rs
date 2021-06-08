// Minimal custom client example.
use k8s_openapi::api::core::v1::Pod;

use kube::{client::ConfigExt, Api, Client, Config, ResourceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;
    let https = config.native_tls_https_connector()?;
    let service = tower::ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(config.auth_layer()?)
        .service(hyper::Client::builder().build(https));
    let client = Client::new(service, config.default_namespace);

    let pods: Api<Pod> = Api::default_namespaced(client);
    for p in pods.list(&Default::default()).await? {
        println!("{}", p.name());
    }

    Ok(())
}
