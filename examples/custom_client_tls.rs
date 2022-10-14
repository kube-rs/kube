// Custom client supporting both openssl-tls and rustls-tls
// Must enable `rustls-tls` feature to run this.
// Run with `USE_RUSTLS=1` to pick rustls.
use k8s_openapi::api::core::v1::Pod;
use tower::ServiceBuilder;
use tracing::*;

use kube::{client::ConfigExt, Api, Client, Config, ResourceExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;

    // Pick TLS at runtime
    let use_rustls = std::env::var("USE_RUSTLS").map(|s| s == "1").unwrap_or(false);
    let client = if use_rustls {
        let https = config.rustls_https_connector()?;
        let service = ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        Client::new(service, config.default_namespace)
    } else {
        let https = config.openssl_https_connector()?;
        let service = ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .service(hyper::Client::builder().build(https));
        Client::new(service, config.default_namespace)
    };

    let pods: Api<Pod> = Api::default_namespaced(client);
    for p in pods.list(&Default::default()).await? {
        info!("{}", p.name_any());
    }

    Ok(())
}
