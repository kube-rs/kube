// Custom client supporting both native-tls and rustls-tls
// Must enable `rustls-tls` feature to run this.
// Run with `USE_RUSTLS=1` to pick rustls.
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
        let https = config.rustls_tls_https_connector()?;
        Client::new(
            ServiceBuilder::new()
                .layer(SetBaseUriLayer::new(config.cluster_url))
                .service(hyper::Client::builder().build(https)),
        )
    } else {
        let https = config.native_tls_https_connector()?;
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
