use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client, ResourceExt,
};
use tracing::*;

use hyper::{body, Body, Request};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let p: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": {
            "containers": [{
                "name": "nginx",
                "image": "nginx",
            }],
        }
    }))?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    // Stop on error including a pod already exists or is still being deleted.
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    let running = await_condition(pods.clone(), "example", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), running).await?;

    let mut pf = pods.portforward("example", &[80]).await?;
    let port = pf.take_stream(80).unwrap();

    // let hyper drive the HTTP state in our DuplexStream via a task
    let (mut sender, connection) = hyper::client::conn::handshake(port).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            warn!("Error in connection: {}", e);
        }
    });

    let http_req = Request::builder()
        .uri("/")
        .header("Connection", "close")
        .header("Host", "127.0.0.1")
        .method("GET")
        .body(Body::from(""))
        .unwrap();

    let (parts, body) = sender.send_request(http_req).await?.into_parts();
    assert!(parts.status == 200);

    let body_bytes = body::to_bytes(body).await?;
    let body_str = std::str::from_utf8(&body_bytes)?;
    assert!(body_str.contains("Welcome to nginx!"));

    // Delete it
    info!("deleting");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}
