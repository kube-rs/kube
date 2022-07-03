use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use tracing::*;

use kube::{
    api::{Api, DeleteParams, PostParams},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client, ResourceExt,
};

use tokio::io::AsyncWriteExt;

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
    let mut port = pf.take_stream(80).unwrap();
    port.write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n")
        .await?;
    let mut rstream = tokio_util::io::ReaderStream::new(port);
    if let Some(res) = rstream.next().await {
        match res {
            Ok(bytes) => {
                let response = std::str::from_utf8(&bytes[..]).unwrap();
                info!("resp: {}", response);
                assert!(response.contains("Welcome to nginx!"));
            }
            Err(err) => warn!("{:?}", err),
        }
    }

    // Delete it
    info!("deleting");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}
