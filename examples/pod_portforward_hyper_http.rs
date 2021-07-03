#[macro_use] extern crate log;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, DeleteParams, ListParams, PostParams, WatchEvent},
    Client, ResourceExt,
};

use hyper::{body, client, Body, Request};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

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

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    // Stop on error including a pod already exists or is still being deleted.
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    let lp = ListParams::default().fields("metadata.name=example").timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", o.name());
            }
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                if s.phase.clone().unwrap_or_default() == "Running" {
                    break;
                }
            }
            _ => {}
        }
    }

    let mut pf = pods.portforward("example", &[80]).await?;
    let mut ports = pf.ports().unwrap();
    let port = ports[0].stream().unwrap();

    let (mut sender, connection) = client::conn::handshake(port).await?;

    // spawn a task to poll the connection and drive the HTTP state
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Error in connection: {}", e);
        }
    });

    let http_req = Request::builder()
        .uri("/")
        .header("Connection", "close")
        .header("Host", "127.0.0.1")
        .method("GET")
        .body(Body::from(""))
        .unwrap();

    let (parts, body) = sender.send_request(http_req).await.unwrap().into_parts();
    assert!(parts.status == 200);

    let body_bytes = body::to_bytes(body).await.unwrap();
    let body_str = std::str::from_utf8(&body_bytes).unwrap();
    assert!(body_str.contains("Welcome to nginx!"));

    // Delete it
    println!("deleting");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name(), "example");
        });

    Ok(())
}
