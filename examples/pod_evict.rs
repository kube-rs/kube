#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use serde_json::json;

use kube::{
    api::{Api, EvictParams, ListParams, PostParams, ResourceExt, WatchEvent},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    // Create a Job
    let pod_name = "empty-pod";
    let empty_pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": pod_name,
        },
        "spec": {
            "containers": [{
                "name": "empty",
                "image": "alpine:latest",
                "command": ["tail", "-f", "/dev/null"]
            }],
        }
    }))?;

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    let pp = PostParams::default();
    pods.create(&pp, &empty_pod).await?;

    // Wait until the pod is running, although it's not necessary
    let lp = ListParams::default()
        .fields("metadata.name=empty-pod")
        .timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", o.name());
            }
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                if s.phase.clone().unwrap_or_default() == "Running" {
                    info!("Ready to evict to {}", o.name());
                    break;
                }
            }
            _ => {}
        }
    }

    // Evict the pod
    let ep = EvictParams::default();
    let eres = pods.evict(pod_name, &ep).await?;
    println!("{:?}", eres);
    Ok(())
}
