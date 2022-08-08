use std::io::Write;
use tracing::*;

use futures::{join, stream, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{
        Api, AttachParams, AttachedProcess, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent,
    },
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    info!("Creating a Pod that outputs numbers for 15s");
    let p: Pod = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "example" },
        "spec": {
            "containers": [{
                "name": "example",
                "image": "alpine",
                "command": ["sh", "-c", "for i in `seq 15`; do if [ $i -lt 7 ]; then echo \"o $i\"; else echo \"e $i\" 1>&2; fi; sleep 1; done;"],
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

    let ap = AttachParams::default();
    // Attach to see numbers printed on stdout.
    let attached = pods.attach("example", &ap).await?;
    // Separate stdout/stderr outputs
    separate_outputs(attached).await;
    // Combining stdout and stderr output.
    // combined_output(proc).await;

    // Delete it
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}

#[allow(dead_code)]
async fn separate_outputs(mut attached: AttachedProcess) {
    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let stdouts = stdout.for_each(|res| async {
        if let Ok(bytes) = res {
            let out = std::io::stdout();
            out.lock().write_all(&bytes).unwrap();
        }
    });
    let stderr = tokio_util::io::ReaderStream::new(attached.stderr().unwrap());
    let stderrs = stderr.for_each(|res| async {
        if let Ok(bytes) = res {
            let out = std::io::stderr();
            out.lock().write_all(&bytes).unwrap();
        }
    });

    join!(stdouts, stderrs);
    if let Some(status) = attached.take_status().unwrap().await {
        info!("{:?}", status);
    }
}

#[allow(dead_code)]
async fn combined_output(mut attached: AttachedProcess) {
    let stdout = tokio_util::io::ReaderStream::new(attached.stdout().unwrap());
    let stderr = tokio_util::io::ReaderStream::new(attached.stderr().unwrap());
    let outputs = stream::select(stdout, stderr).for_each(|res| async {
        if let Ok(bytes) = res {
            let out = std::io::stdout();
            out.lock().write_all(&bytes).unwrap();
        }
    });
    outputs.await;
    if let Some(status) = attached.take_status().unwrap().await {
        info!("{:?}", status);
    }
}
