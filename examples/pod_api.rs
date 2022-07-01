use k8s_openapi::api::core::v1::Pod;
use serde_json::json;
use tracing::*;

use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
    runtime::wait::{await_condition, conditions::is_pod_running},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // Manage pods
    let pods: Api<Pod> = Api::default_namespaced(client);

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
            let name = o.name_any();
            assert_eq!(p.name_any(), name);
            info!("Created {}", name);
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),                        // any other case is probably bad
    }

    // Watch it phase for a few seconds
    let establish = await_condition(pods.clone(), "blog", is_pod_running());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(15), establish).await?;

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
        info!("Found Pod: {}", p.name_any());
    }

    // Delete it
    let dp = DeleteParams::default();
    pods.delete("blog", &dp).await?.map_left(|pdel| {
        assert_eq!(pdel.name_any(), "blog");
        info!("Deleting blog pod started: {:?}", pdel);
    });

    Ok(())
}
