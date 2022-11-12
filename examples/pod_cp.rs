use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use tracing::*;

use kube::{
    api::{Api, AttachParams, DeleteParams, ListParams, PostParams, ResourceExt, WatchEvent},
    Client,
};
use tokio::io::AsyncWriteExt;

// A `kubectl cp` analog example.

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
    // Stop on error including a pod already exists or still being deleted.
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

    let data = "data for pod";
    let file_name = "foo.txt";

    // Write the data to pod
    {
        let mut header = tar::Header::new_gnu();
        header.set_path(file_name).unwrap();
        header.set_size(data.len() as u64);
        header.set_cksum();

        let mut ar = tar::Builder::new(Vec::new());
        ar.append(&header, &mut data.as_bytes()).unwrap();
        let data = ar.into_inner().unwrap();

        let ap = AttachParams::default().stdin(true).stderr(false);
        let mut tar = pods
            .exec("example", vec!["tar", "xf", "-", "-C", "/"], &ap)
            .await?;
        tar.stdin().unwrap().write_all(&data).await?;
    }

    // Check that the file was written
    {
        let ap = AttachParams::default().stderr(false);
        let mut cat = pods
            .exec("example", vec!["cat", &format!("/{}", file_name)], &ap)
            .await?;
        let mut cat_out = tokio_util::io::ReaderStream::new(cat.stdout().unwrap());
        let next_stdout = cat_out.next().await.unwrap()?;

        info!("Contents of the file on the pod: {:?}", next_stdout);
        assert_eq!(next_stdout, data);
    }

    // Clean up the pod
    pods.delete("example", &DeleteParams::default())
        .await?
        .map_left(|pdel| {
            assert_eq!(pdel.name_any(), "example");
        });

    Ok(())
}
