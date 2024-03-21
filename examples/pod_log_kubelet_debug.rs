use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use tracing::*;

use futures::AsyncBufReadExt;
use hyper::Uri;
use kube::{
    api::{Api, DeleteParams, ResourceExt},
    core::{node_proxy::KubeletDebugParams, subresource::LogParams},
    Client, Config,
};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let pods: Api<Pod> = Api::default_namespaced(client);

    // create busybox pod that's alive for at most 30s
    let p: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": "example",
            "labels": { "app": "kube-rs-test" },
        },
        "spec": {
            "terminationGracePeriodSeconds": 1,
            "restartPolicy": "Never",
            "containers": [{
              "name": "busybox",
              "image": "busybox:1.34.1",
              "command": ["sh", "-c", "for i in $(seq 1 5); do echo kube $i; sleep 0.1; done"],
            }],
        }
    }))?;

    match pods.create(&Default::default(), &p).await {
        Ok(o) => assert_eq!(p.name_unchecked(), o.name_unchecked()),
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if we failed to clean-up
        Err(e) => return Err(e.into()),                        // any other case if a failure
    }

    // wait for container to finish
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    // Create a client for node proxy
    let mut config = Config::infer().await?;
    config.accept_invalid_certs = true;
    config.cluster_url = "https://localhost:10250".to_string().parse::<Uri>().unwrap();
    let client: Client = config.try_into()?;

    // Get logs directly from the node, bypassing the kube-apiserver
    let lp = LogParams::default();
    let mut logs_stream = client
        .node_logs(
            &KubeletDebugParams {
                name: "example",
                namespace: "default",
                ..Default::default()
            },
            "busybox",
            &lp,
        )
        .await?
        .lines();


    while let Some(line) = logs_stream.try_next().await? {
        println!("{line}");
    }

    // Delete it
    info!("deleting");
    let _ = pods.delete("example", &DeleteParams::default()).await?;

    Ok(())
}
