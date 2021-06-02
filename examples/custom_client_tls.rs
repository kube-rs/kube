// Custom client supporting both native-tls and rustls-tls
// Run with `USE_RUSTLS=1` to pick rustls.
use std::sync::Arc;

use futures::{StreamExt, TryStreamExt};
use hyper::client::HttpConnector;
use k8s_openapi::api::core::v1::Pod;
use serde_json::json;
use tower::ServiceBuilder;

use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent},
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

    // Manage pods
    let pods: Api<Pod> = Api::namespaced(client, "default");
    // Create pod
    let p: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": { "containers": [{ "name": "example", "image": "alpine" }] }
    }))?;

    let pp = PostParams::default();
    match pods.create(&pp, &p).await {
        Ok(o) => {
            let name = o.name();
            assert_eq!(p.name(), name);
            tracing::info!("Created {}", name);
            std::thread::sleep(std::time::Duration::from_millis(5_000));
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),
    }

    // Watch it phase for a few seconds
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", "example"))
        .timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => tracing::info!("Added {}", o.name()),
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                let phase = s.phase.clone().unwrap_or_default();
                tracing::info!("Modified: {} with phase: {}", o.name(), phase);
            }
            WatchEvent::Deleted(o) => tracing::info!("Deleted {}", o.name()),
            WatchEvent::Error(e) => tracing::error!("Error {}", e),
            _ => {}
        }
    }

    if let Some(spec) = &pods.get("example").await?.spec {
        assert_eq!(spec.containers[0].name, "example");
    }

    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name(), "example");
        });

    Ok(())
}
