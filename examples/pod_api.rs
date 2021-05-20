#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use serde_json::json;

use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt, WatchEvent},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // Manage pods
    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    for p in pods.list(&Default::default()).await? {
        use kube::ResourceExt;
        info!("found pod {}", p.name());
    }

    // Create Pod blog
    info!("Creating Pod instance blog");
    let p: Pod = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "blog" },
        "spec": {
            "containers": [{
              "name": "blog",
              "image": "clux/blog:0.1.0"
            }],
        }
    }))?;

    let pp = PostParams::default();
    match pods.create(&pp, &p).await {
        Ok(o) => {
            let name = o.name();
            assert_eq!(p.name(), name);
            info!("Created {}", name);
            // wait for it..
            std::thread::sleep(std::time::Duration::from_millis(5_000));
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),                        // any other case is probably bad
    }

    // Watch it phase for a few seconds
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", "blog"))
        .timeout(10);
    let mut stream = pods.watch(&lp, "0").await?.boxed();
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(o) => info!("Added {}", o.name()),
            WatchEvent::Modified(o) => {
                let s = o.status.as_ref().expect("status exists on pod");
                let phase = s.phase.clone().unwrap_or_default();
                info!("Modified: {} with phase: {}", o.name(), phase);
            }
            WatchEvent::Deleted(o) => info!("Deleted {}", o.name()),
            WatchEvent::Error(e) => error!("Error {}", e),
            _ => {}
        }
    }

    // Verify we can get it
    info!("Get Pod blog");
    let p1cpy = pods.get("blog").await?;
    if let Some(spec) = &p1cpy.spec {
        info!("Got blog pod with containers: {:?}", spec.containers);
        assert_eq!(spec.containers[0].name, "blog");
    }

    // Replace its spec
    info!("Patch Pod blog");
    let patch = json!({
        "metadata": {
            "resourceVersion": p1cpy.resource_version(),
        },
        "spec": {
            "activeDeadlineSeconds": 5
        }
    });
    let patchparams = PatchParams::default();
    let p_patched = pods.patch("blog", &patchparams, &Patch::Merge(&patch)).await?;
    assert_eq!(p_patched.spec.unwrap().active_deadline_seconds, Some(5));

    let lp = ListParams::default().fields(&format!("metadata.name={}", "blog")); // only want results for our pod
    for p in pods.list(&lp).await? {
        info!("Found Pod: {}", p.name());
    }

    // Delete it
    let dp = DeleteParams::default();
    pods.delete("blog", &dp).await?.map_left(|pdel| {
        assert_eq!(pdel.name(), "blog");
        info!("Deleting blog pod started: {:?}", pdel);
    });

    Ok(())
}
