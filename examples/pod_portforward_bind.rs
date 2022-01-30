// Example to listen on port 8080 locally, forwarding to port 80 in the example pod.
// Similar to `kubectl port-forward pod/example 8080:80`.
use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use futures::FutureExt;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use tokio::sync::Mutex;
use tower::ServiceExt;

use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client, ResourceExt,
};

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
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), running).await?;

    // Get `Portforwarder` that handles the WebSocket connection.
    // There's no need to spawn a task to drive this, but it can be awaited to be notified on error.
    let mut forwarder = pods.portforward("example", &[80]).await?;
    let port = forwarder.ports()[0].stream().unwrap();

    // let hyper drive the HTTP state in our DuplexStream via a task
    let (sender, connection) = hyper::client::conn::handshake(port).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            log::error!("error in connection: {}", e);
        }
    });
    // The following task is only used to show any error from the forwarder.
    // This example can be stopped with Ctrl-C if anything happens.
    tokio::spawn(async move {
        if let Err(e) = forwarder.await {
            log::error!("forwarder errored: {}", e);
        }
    });

    // Shared `SendRequest<Body>` to relay the request.
    let context = Arc::new(Mutex::new(sender));
    let make_service = make_service_fn(move |_conn| {
        let context = context.clone();
        let service = service_fn(move |req| handle(context.clone(), req));
        async move { Ok::<_, Infallible>(service) }
    });

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let server = Server::bind(&addr)
        .serve(make_service)
        .with_graceful_shutdown(async {
            rx.await.ok();
        });
    println!("Forwarding http://{} to port 80 in the pod", addr);
    println!("Try opening http://{0} in a browser, or `curl http://{0}`", addr);
    println!("Use Ctrl-C to stop the server and delete the pod");
    // Stop the server and delete the pod on Ctrl-C.
    tokio::spawn(async move {
        tokio::signal::ctrl_c().map(|_| ()).await;
        log::info!("stopping the server");
        let _ = tx.send(());
    });
    if let Err(e) = server.await {
        log::error!("server error: {}", e);
    }

    log::info!("deleting the pod");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name(), "example");
        });

    Ok(())
}

// Simply forwards the request to the port through the shared `SendRequest<Body>`.
async fn handle(
    context: Arc<Mutex<hyper::client::conn::SendRequest<hyper::Body>>>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let mut sender = context.lock().await;
    let response = sender.ready().await.unwrap().send_request(req).await.unwrap();
    Ok(response)
}
