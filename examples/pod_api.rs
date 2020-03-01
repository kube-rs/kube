#[macro_use] extern crate log;
use serde_json::json;

use kube::{
    api::{Api, DeleteParams, ListParams, PatchParams, PostParams},
    client::APIClient,
    config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);

    // Manage pods
    let pods = Api::v1Pod(client).within("default");

    // Create Pod blog
    info!("Creating Pod instance blog");
    let p = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": { "name": "blog" },
        "spec": {
            "containers": [{
              "name": "blog",
              "image": "clux/blog:0.1.0"
            }],
        }
    });

    let pp = PostParams::default();
    match pods.create(&pp, serde_json::to_vec(&p)?).await {
        Ok(o) => {
            assert_eq!(p["metadata"]["name"], o.metadata.name);
            info!("Created {}", o.metadata.name);
            // wait for it..
            std::thread::sleep(std::time::Duration::from_millis(5_000));
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),                        // any other case is probably bad
    }

    // Verify we can get it
    info!("Get Pod blog");
    let p1cpy = pods.get("blog").await?;
    println!("Got blog pod with containers: {:?}", p1cpy.spec.containers);
    assert_eq!(p1cpy.spec.containers[0].name, "blog");

    // Replace its spec
    info!("Patch Pod blog");
    let patch = json!({
        "metadata": {
            "resourceVersion": p1cpy.metadata.resourceVersion,
        },
        "spec": {
            "activeDeadlineSeconds": 5
        }
    });
    let patch_params = PatchParams::default();
    let p_patched = pods
        .patch("blog", &patch_params, serde_json::to_vec(&patch)?)
        .await?;
    assert_eq!(p_patched.spec.active_deadline_seconds, Some(5));

    for p in pods.list(&ListParams::default()).await? {
        println!("Got Pod: {}", p.metadata.name);
    }

    // Delete it
    let dp = DeleteParams::default();
    pods.delete("blog", &dp).await?.map_left(|pdel| {
        assert_eq!(pdel.metadata.name, "blog");
        info!("Deleting blog pod started");
    });

    Ok(())
}
