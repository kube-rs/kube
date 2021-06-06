// Custom client supporting both native-tls and rustls-tls
// Must enable `rustls-tls` feature to run this.
// Run with `USE_RUSTLS=1` to pick rustls.
use k8s_openapi::api::core::v1::Pod;
use tower::ServiceBuilder;

use kube::{
    Api, ResourceExt,
    client::ConfigExt,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;

    // Pick TLS at runtime
    let use_rustls = std::env::var("USE_RUSTLS").map(|s| s == "1").unwrap_or(false);
    let client = (if use_rustls {
        let https = config.rustls_https_connector()?;
        Client::new(
            ServiceBuilder::new()
                .layer(config.base_uri_layer())
                .service(hyper::Client::builder().build(https)),
        )
    } else {
        let https = config.native_tls_https_connector()?;
        Client::new(
            ServiceBuilder::new()
                .layer(config.base_uri_layer())
                .service(hyper::Client::builder().build(https)),
        )
    }).with_default_namespace(config.default_ns);

    let pods: Api<Pod> = Api::default_namespaced(client);
    for p in pods.list(&Default::default()).await? {
        println!("{}", p.name());
    }

    Ok(())
}
