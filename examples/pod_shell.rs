use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use tracing::*;

use kube::{
    api::{Api, AttachParams, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent},
    Client,
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
                "name": "example",
                "image": "alpine",
                // Do nothing
                "command": ["tail", "-f", "/dev/null"],
            }],
        }
    }))?;

    let pods: Api<Pod> = Api::default_namespaced(client);
    // Stop on error including a pod already exists or is still being deleted.
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    let lp = ListParams::default().fields("metadata.name=example").timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", o.name_any());
            }
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                if s.phase.clone().unwrap_or_default() == "Running" {
                    info!("Ready to attach to {}", o.name_any());
                    break;
                }
            }
            _ => {}
        }
    }

    // Do an interactive exec to a blog pod with the `sh` command
    let ap = AttachParams::interactive_tty();
    let mut attached = pods.exec("example", vec!["sh"], &ap).await?;

    // The received streams from `AttachedProcess`
    let mut stdin_writer = attached.stdin().unwrap();
    let mut stdout_reader = attached.stdout().unwrap();

    // > For interactive uses, it is recommended to spawn a thread dedicated to user input and use blocking IO directly in that thread.
    // > https://docs.rs/tokio/0.2.24/tokio/io/fn.stdin.html
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    // pipe current stdin to the stdin writer from ws
    tokio::spawn(async move {
        tokio::io::copy(&mut stdin, &mut stdin_writer).await.unwrap();
    });
    // pipe stdout from ws to current stdout
    tokio::spawn(async move {
        tokio::io::copy(&mut stdout_reader, &mut stdout).await.unwrap();
    });
    // When done, type `exit\n` to end it, so the pod is deleted.
    let status = attached.take_status().unwrap().await;
    info!("{:?}", status);

    // Delete it
    info!("deleting");
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}
