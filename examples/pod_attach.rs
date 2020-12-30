#[macro_use]
extern crate log;

use std::io::Write;

use futures::{future, join, stream::select, FutureExt, StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;

use kube::{
    api::{Api, AttachParams, AttachedProcess, DeleteParams, ListParams, Meta, PostParams, WatchEvent},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

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

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
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
            assert_eq!(Meta::name(&pdel), "example");
        });

    Ok(())
}

#[allow(dead_code)]
async fn separate_outputs(mut attached: AttachedProcess) {
    let stdout_stream = attached.stdout().take().unwrap();
    let stdouts = stdout_stream
        .for_each(|out| {
            if let Ok(bytes) = out {
                let out = std::io::stdout();
                out.lock().write_all(&bytes[..]).unwrap();
            }
            future::ready(())
        })
        .fuse();
    let stderr_stream = attached.stderr().take().unwrap();
    let stderrs = stderr_stream
        .for_each(|out| {
            if let Ok(bytes) = out {
                let out = std::io::stderr();
                out.lock().write_all(&bytes[..]).unwrap();
            }
            future::ready(())
        })
        .fuse();

    join!(stdouts, stderrs);

    if let Some(status) = attached.await {
        println!("{:?}", status);
    }
}

#[allow(dead_code)]
async fn combined_output(mut attached: AttachedProcess) {
    let stdout_stream = attached.stdout().take().unwrap();
    let stderr_stream = attached.stderr().take().unwrap();
    let outputs = select(stdout_stream, stderr_stream).for_each(|out| {
        if let Ok(bytes) = out {
            let out = std::io::stdout();
            out.lock().write_all(&bytes[..]).unwrap();
        }
        future::ready(())
    });
    outputs.await;

    if let Some(status) = attached.await {
        println!("{:?}", status);
    }
}
