use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions::is_pod_running},
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
    let running = await_condition(pods.clone(), "example", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), running).await?;

    let mut pf = pods.portforward("example", &[80]).await?;
    let ports = pf.ports();
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

    let (parts, body) = sender.send_request(http_req).await?.into_parts();
    assert!(parts.status == 200);

    let body_bytes = body::to_bytes(body).await?;
    let body_str = std::str::from_utf8(&body_bytes)?;
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
