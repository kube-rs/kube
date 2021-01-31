#[macro_use] extern crate log;

use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, AttachParams, AttachedProcess, DeleteParams, ListParams, Meta, PostParams, WatchEvent},
    Client,
};
use tokio::io::AsyncWriteExt;

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
                "name": "example",
                "image": "alpine",
                // Do nothing
                "command": ["tail", "-f", "/dev/null"],
            }],
        }
    }))?;

    let mut pods: Api<Pod> = Api::namespaced(client, &namespace);
    // Stop on error including a pod already exists or is still being deleted.
    pods.create(&PostParams::default(), &p).await?;

    // Wait until the pod is running, otherwise we get 500 error.
    let lp = ListParams::default().fields("metadata.name=example").timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => {
                info!("Added {}", Meta::name(&o));
            }
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                if s.phase.clone().unwrap_or_default() == "Running" {
                    info!("Ready to attach to {}", Meta::name(&o));
                    break;
                }
            }
            _ => {}
        }
    }

    // These examples are mostly taken from Python client's integration tests.
    {
        let attached = pods
            .exec(
                "example",
                vec!["sh", "-c", "for i in $(seq 1 3); do date; done"],
                &AttachParams::default().stderr(false),
            )
            .await?;
        let output = get_output(attached).await;
        println!("{}", output);
        assert_eq!(output.lines().count(), 3);
    }

    {
        let attached = pods
            .exec("example", vec!["uptime"], &AttachParams::default().stderr(false))
            .await?;
        let output = get_output(attached).await;
        println!("{}", output);
        assert_eq!(output.lines().count(), 1);
    }

    // Stdin example
    {
        let mut attached = pods
            .exec(
                "example",
                vec!["sh"],
                &AttachParams::default().stdin(true).stderr(false),
            )
            .await?;
        let mut stdin_writer = attached.stdin().unwrap();
        let mut stdout_stream = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
        let next_stdout = stdout_stream.next();
        stdin_writer.write(b"echo test string 1\n").await?;
        let stdout = String::from_utf8(next_stdout.await.unwrap().unwrap().to_vec()).unwrap();
        println!("{}", stdout);
        assert_eq!(stdout, "test string 1\n");

        // AttachedProcess resolves with status object.
        // Send `exit 1` to get a failure status.
        stdin_writer.write(b"exit 1\n").await?;
        if let Some(status) = attached.await {
            println!("{:?}", status);
            assert_eq!(status.status, Some("Failure".to_owned()));
            assert_eq!(status.reason, Some("NonZeroExitCode".to_owned()));
        }
    }

    // Delete it
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(Meta::name(&pdel), "example");
        });

    Ok(())
}

async fn get_output(mut attached: AttachedProcess) -> String {
    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let out = stdout
        .filter_map(|r| async { r.ok().and_then(|v| String::from_utf8(v.to_vec()).ok()) })
        .collect::<Vec<_>>()
        .await
        .join("");
    attached.await;
    out
}
