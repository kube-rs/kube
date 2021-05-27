// Run with `cargo run --example custom_client --no-default-features --features native-tls,rustls-tls`
#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector as RustlsHttpsConnector;
use hyper_tls::HttpsConnector;
use k8s_openapi::api::core::v1::Pod;
use serde_json::json;
use tokio_native_tls::TlsConnector;
use tower::ServiceBuilder;

use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent},
    service::SetBaseUriLayer,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();

    let config = Config::infer().await?;
    let cluster_url = config.cluster_url.clone();
    let common = ServiceBuilder::new()
        .layer(SetBaseUriLayer::new(cluster_url))
        .into_inner();
    let mut http = HttpConnector::new();
    http.enforce_http(false);

    // Pick TLS at runtime
    let use_rustls = std::env::var("USE_RUSTLS").map(|s| s == "1").unwrap_or(false);
    let client = if use_rustls {
        let https =
            RustlsHttpsConnector::from((http, std::sync::Arc::new(config.rustls_tls_client_config()?)));
        let inner = ServiceBuilder::new()
            .layer(common)
            .service(hyper::Client::builder().build(https));
        Client::new(inner)
    } else {
        let https = HttpsConnector::from((http, TlsConnector::from(config.native_tls_connector()?)));
        let inner = ServiceBuilder::new()
            .layer(common)
            .service(hyper::Client::builder().build(https));
        Client::new(inner)
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
            info!("Created {}", name);
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
            WatchEvent::Added(o) => info!("Added {}", o.name()),
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                let phase = s.phase.clone().unwrap_or_default();
                info!("Modified: {} with phase: {}", o.name(), phase);
            }
            WatchEvent::Deleted(o) => info!("Deleted {}", o.name()),
            WatchEvent::Error(e) => error!("Error {}", e),
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
