#[macro_use] extern crate log;
use either::Either::{Left, Right};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tokio::time::sleep;

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;

use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt},
    core::crd::v1beta1::CustomResourceExt,
    Client, CustomResource,
};

// Own custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
#[kube(apiextensions = "v1beta1")]
#[kube(status = "FooStatus")]
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
#[kube(printcolumn = r#"{"name":"Team", "jsonPath": ".spec.metadata.team", "type": "string"}"#)]
pub struct FooSpec {
    name: String,
    info: String,
    replicas: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct FooStatus {
    is_bad: bool,
    replicas: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    // Manage CRDs first
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());

    // Delete any old versions of it first:
    let dp = DeleteParams::default();
    // but ignore delete err if not exists
    let _ = crds.delete("foos.clux.dev", &dp).await.map(|res| {
        res.map_left(|o| {
            info!(
                "Deleting {}: ({:?})",
                o.name(),
                o.status.unwrap().conditions.last()
            );
        })
        .map_right(|s| {
            // it's gone.
            info!("Deleted foos.clux.dev: ({:?})", s);
        })
    });
    // Wait for the delete to take place (map-left case or delete from previous run)
    sleep(Duration::from_secs(2)).await;

    // Create the CRD so we can create Foos in kube
    let foocrd = Foo::crd();
    info!("Creating Foo CRD: {}", serde_json::to_string_pretty(&foocrd)?);
    let pp = PostParams::default();
    let patch_params = PatchParams::default();
    match crds.create(&pp, &foocrd).await {
        Ok(o) => {
            info!("Created {} ({:?})", o.name(), o.status.unwrap());
            debug!("Created CRD: {:?}", o.spec);
        }
        Err(kube::Error::Api(ae)) => assert_eq!(ae.code, 409), // if you skipped delete, for instance
        Err(e) => return Err(e.into()),                        // any other case is probably bad
    }
    // Wait for the api to catch up
    sleep(Duration::from_secs(1)).await;

    // Manage the Foo CR
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    // Create Foo baz
    info!("Creating Foo instance baz");
    let f1 = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: "old baz".into(),
        replicas: 1,
    });
    let o = foos.create(&pp, &f1).await?;
    assert_eq!(ResourceExt::name(&f1), ResourceExt::name(&o));
    info!("Created {}", o.name());

    // Verify we can get it
    info!("Get Foo baz");
    let f1cpy = foos.get("baz").await?;
    assert_eq!(f1cpy.spec.info, "old baz");

    // Replace its spec
    info!("Replace Foo baz");
    let foo_replace: Foo = serde_json::from_value(json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "baz",
            // Updates need to provide our last observed version:
            "resourceVersion": f1cpy.resource_version(),
        },
        "spec": { "name": "baz", "info": "new baz", "replicas": 1 },
    }))?;
    let f1_replaced = foos.replace("baz", &pp, &foo_replace).await?;
    assert_eq!(f1_replaced.spec.name, "baz");
    assert_eq!(f1_replaced.spec.info, "new baz");
    assert!(f1_replaced.status.is_none());

    // Delete it
    foos.delete("baz", &dp).await?.map_left(|f1del| {
        assert_eq!(f1del.spec.info, "old baz");
    });

    // Create Foo qux with status
    info!("Create Foo instance qux");
    let mut f2 = Foo::new("qux", FooSpec {
        name: "qux".into(),
        replicas: 0,
        info: "unpatched qux".into(),
    });
    f2.status = Some(FooStatus::default());

    let o = foos.create(&pp, &f2).await?;
    info!("Created {}", o.name());

    // Update status on qux
    info!("Replace Status on Foo instance qux");
    let fs = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": {
            "name": "qux",
            // Updates need to provide our last observed version:
            "resourceVersion": o.resource_version(),
        },
        "status": FooStatus { is_bad: true, replicas: 0 }
    });
    let o = foos.replace_status("qux", &pp, serde_json::to_vec(&fs)?).await?;
    info!("Replaced status {:?} for {}", o.status, o.name());
    assert!(o.status.unwrap().is_bad);

    info!("Patch Status on Foo instance qux");
    let fs = json!({
        "status": FooStatus { is_bad: false, replicas: 1 }
    });
    let o = foos
        .patch_status("qux", &patch_params, &Patch::Merge(&fs))
        .await?;
    info!("Patched status {:?} for {}", o.status, o.name());
    assert!(!o.status.unwrap().is_bad);

    info!("Get Status on Foo instance qux");
    let o = foos.get_status("qux").await?;
    info!("Got status {:?} for {}", o.status, o.name());
    assert!(!o.status.unwrap().is_bad);

    // Check scale subresource:
    info!("Get Scale on Foo instance qux");
    let scale = foos.get_scale("qux").await?;
    info!("Got scale {:?} - {:?}", scale.spec, scale.status);
    assert_eq!(scale.status.unwrap().replicas, 1);

    // Scale up
    let fs = json!({
        "spec": { "replicas": 2 }
    });
    let o = foos.patch_scale("qux", &patch_params, &Patch::Merge(&fs)).await?;
    info!("Patched scale {:?} for {}", o.spec, o.name());
    assert_eq!(o.status.unwrap().replicas, 1);
    assert_eq!(o.spec.unwrap().replicas.unwrap(), 2); // we only asked for more

    // Modify a Foo qux with a Patch
    info!("Patch Foo instance qux");
    let patch = json!({
        "spec": { "info": "patched qux" }
    });
    let o = foos.patch("qux", &patch_params, &Patch::Merge(&patch)).await?;
    info!("Patched {} with new name: {}", o.name(), o.spec.name);
    assert_eq!(o.spec.info, "patched qux");
    assert_eq!(o.spec.name, "qux"); // didn't blat existing params

    // Check we have 1 remaining instance
    let lp = ListParams::default();
    let res = foos.list(&lp).await?;
    assert_eq!(res.items.len(), 1);

    // Delete the last - expect a status back (instant delete)
    assert!(foos.delete("qux", &dp).await?.is_right());

    // Cleanup the full collection - expect a wait
    match foos.delete_collection(&dp, &lp).await? {
        Left(list) => {
            let deleted: Vec<_> = list.iter().map(ResourceExt::name).collect();
            info!("Deleting collection of foos: {:?}", deleted);
        }
        Right(status) => {
            info!("Deleted collection of crds: status={:?}", status);
        }
    }

    // Cleanup the CRD definition
    match crds.delete("foos.clux.dev", &dp).await? {
        Left(o) => {
            info!(
                "Deleting {} CRD definition: {:?}",
                o.name(),
                o.status.unwrap().conditions.last()
            );
        }
        Right(status) => {
            info!("Deleted foos CRD definition: status={:?}", status);
        }
    }

    Ok(())
}
