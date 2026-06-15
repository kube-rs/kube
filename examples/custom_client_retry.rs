// Custom client with retry layer example.
//
// This demonstrates how to add a retry layer to the kube client
// that automatically retries requests on transient failures (429, 503, 504).

use hyper_util::rt::TokioExecutor;
use k8s_openapi::api::core::v1::Pod;
use tower::{BoxError, ServiceBuilder, buffer::BufferLayer, retry::RetryLayer};
use tracing::*;

use kube::{
    Api, Client, Config, ResourceExt,
    client::{ConfigExt, retry::RetryPolicy},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::infer().await?;

    let https = config.rustls_https_connector()?;

    // Build a custom service stack with retry support.
    //
    // Layer order matters:
    // - BufferLayer is needed because RetryLayer requires the service to implement Clone
    // - RetryLayer wraps the service and retries on 429, 503, 504 responses
    let service = ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(config.auth_layer()?)
        // BufferLayer provides Clone capability required by RetryLayer
        .layer(BufferLayer::new(1024))
        // RetryLayer with default policy: 500ms-5s backoff, max 3 retries
        .layer(RetryLayer::new(RetryPolicy::default()))
        .map_err(BoxError::from)
        .service(hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https));

    let client = Client::new(service, config.default_namespace);

    // Use the client as normal - retries happen automatically
    let pods: Api<Pod> = Api::default_namespaced(client);
    for p in pods.list(&Default::default()).await? {
        info!("{}", p.name_any());
    }

    Ok(())
}
