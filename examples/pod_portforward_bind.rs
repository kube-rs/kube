use anyhow::Context;
// Example to listen on port 8080 locally, forwarding to port 80 in the example pod.
// Similar to `kubectl port-forward pod/example 8080:80`.
use futures::{StreamExt, TryStreamExt};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};
use tokio_stream::wrappers::TcpListenerStream;
use tracing::*;

use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client, ResourceExt,
};

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
    info!("creating nginx pod");
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    info!("waiting for nginx pod to start");
    let running = await_condition(pods.clone(), "example", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(30), running).await?;

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let pod_port = 80;
    info!(local_addr = %addr, pod_port, "forwarding traffic to the pod");
    info!("try opening http://{0} in a browser, or `curl http://{0}`", addr);
    info!("use Ctrl-C to stop the server and delete the pod");
    let server = TcpListenerStream::new(TcpListener::bind(addr).await.unwrap())
        .take_until(tokio::signal::ctrl_c())
        .try_for_each(|client_conn| async {
            if let Ok(peer_addr) = client_conn.peer_addr() {
                info!(%peer_addr, "new connection");
            }
            let pods = pods.clone();
            tokio::spawn(async move {
                if let Err(e) = forward_connection(&pods, "example", 80, client_conn).await {
                    error!(
                        error = e.as_ref() as &dyn std::error::Error,
                        "failed to forward connection"
                    );
                }
            });
            // keep the server running
            Ok(())
        });
    if let Err(e) = server.await {
        error!(error = &e as &dyn std::error::Error, "server error");
    }

    info!("deleting the pod");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin,
) -> anyhow::Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder
        .take_stream(port)
        .context("port not found in forwarder")?;
    tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
    drop(upstream_conn);
    forwarder.join().await?;
    info!("connection closed");
    Ok(())
}
