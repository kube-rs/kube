// Custom client supporting both native-tls and rustls-tls
// Must enable `rustls-tls` feature to run this.
// Run with `USE_RUSTLS=1` to pick rustls.
use std::sync::Arc;

use hyper::client::HttpConnector;
use k8s_openapi::api::core::v1::ConfigMap;
use tower::ServiceBuilder;

use kube::{
    api::{Api, ListParams},
    service::SetBaseUriLayer,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;

    // Pick TLS at runtime
    let use_rustls = std::env::var("USE_RUSTLS").map(|s| s == "1").unwrap_or(false);
    let client = if use_rustls {
        let https = {
            let rustls_config = Arc::new(config.rustls_tls_client_config()?);
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            hyper_rustls::HttpsConnector::from((http, rustls_config))
        };
        Client::new(
            ServiceBuilder::new()
                .layer(SetBaseUriLayer::new(config.cluster_url))
                .service(hyper::Client::builder().build(https)),
        )
    } else {
        let https = {
            let tls = tokio_native_tls::TlsConnector::from(config.native_tls_connector()?);
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            hyper_tls::HttpsConnector::from((http, tls))
        };
        Client::new(
            ServiceBuilder::new()
                .layer(SetBaseUriLayer::new(config.cluster_url))
                .service(hyper::Client::builder().build(https)),
        )
    };

    let cms: Api<ConfigMap> = Api::namespaced(client, "default");
    for cm in cms.list(&ListParams::default()).await? {
        println!("{:?}", cm);
    }

    Ok(())
}
