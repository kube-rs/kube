//! Exercises the generic owner-reference and finalizer helpers from `kube-core`
//! (see <https://github.com/kube-rs/kube/issues/428>) against a real apiserver.
//!
//! Demonstrates:
//! - `set_controller_reference` / `set_owner_reference` and the `AlreadyOwnedError` conflict check
//! - the apiserver's garbage collector cascading a delete via the controller owner reference
//! - `ResourceExt::{has_finalizer, add_finalizer, remove_finalizer}` blocking and then unblocking a delete
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    Client, Resource,
    api::{Api, DeleteParams, Patch, PatchParams, PostParams, ResourceExt},
    core::{AlreadyOwnedError, has_owner_reference, set_controller_reference},
};
use tracing::info;

async fn create_configmap(api: &Api<ConfigMap>, name: &str) -> anyhow::Result<ConfigMap> {
    let cm = serde_json::from_value(serde_json::json!({
        "apiVersion": "v1",
        "kind": "ConfigMap",
        "metadata": { "name": name },
    }))?;
    Ok(api.create(&PostParams::default(), &cm).await?)
}

async fn patch_owner_references(api: &Api<ConfigMap>, cm: &ConfigMap) -> anyhow::Result<ConfigMap> {
    let patch = serde_json::json!({ "metadata": { "ownerReferences": cm.owner_references() } });
    Ok(api
        .patch(&cm.name_any(), &PatchParams::default(), &Patch::Merge(patch))
        .await?)
}

async fn patch_finalizers(api: &Api<ConfigMap>, cm: &ConfigMap) -> anyhow::Result<ConfigMap> {
    let patch = serde_json::json!({ "metadata": { "finalizers": cm.finalizers() } });
    Ok(api
        .patch(&cm.name_any(), &PatchParams::default(), &Patch::Merge(patch))
        .await?)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let cms: Api<ConfigMap> = Api::default_namespaced(client);

    // --- owner reference / garbage collection demo ---
    info!("Creating owner and child ConfigMaps");
    let owner = create_configmap(&cms, "owner-finalizer-demo-owner").await?;
    let mut child = create_configmap(&cms, "owner-finalizer-demo-child").await?;

    set_controller_reference(&owner, &(), &mut child)?;
    let child = patch_owner_references(&cms, &child).await?;
    let owner_uid = owner.uid().expect("created object has a uid");
    assert!(has_owner_reference(&child, &owner_uid));
    info!(
        "Child now has a controller owner reference pointing at owner uid {}",
        owner_uid
    );

    info!("Verifying a conflicting controller is rejected");
    let other_owner = create_configmap(&cms, "owner-finalizer-demo-other-owner").await?;
    let mut child_copy = child.clone();
    let err: AlreadyOwnedError = set_controller_reference(&other_owner, &(), &mut child_copy).unwrap_err();
    assert_eq!(err.existing_uid, owner_uid);
    assert_eq!(err.new_uid, other_owner.uid().expect("created object has a uid"));
    info!("Got expected conflict error: {err}");
    cms.delete("owner-finalizer-demo-other-owner", &DeleteParams::default())
        .await?;

    info!("Deleting owner and waiting for the garbage collector to cascade-delete the child");
    cms.delete("owner-finalizer-demo-owner", &DeleteParams::background())
        .await?;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        if cms.get_opt("owner-finalizer-demo-child").await?.is_none() {
            info!("Child was garbage collected");
            break;
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("timed out waiting for garbage collector to delete the child");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    // --- finalizer demo ---
    info!("Creating a ConfigMap with a finalizer");
    let mut protected = create_configmap(&cms, "owner-finalizer-demo-protected").await?;
    assert!(!protected.has_finalizer("owner-finalizer-demo.kube.rs/cleanup"));
    assert!(protected.add_finalizer("owner-finalizer-demo.kube.rs/cleanup"));
    patch_finalizers(&cms, &protected).await?;

    info!("Deleting it: the apiserver should keep it around until the finalizer is removed");
    cms.delete("owner-finalizer-demo-protected", &DeleteParams::default())
        .await?;
    let still_there = cms
        .get("owner-finalizer-demo-protected")
        .await
        .expect("object with an outstanding finalizer must not be gone yet");
    assert!(still_there.meta().deletion_timestamp.is_some());
    assert!(still_there.has_finalizer("owner-finalizer-demo.kube.rs/cleanup"));
    info!("Confirmed: object is terminating but still present because of the finalizer");

    let mut protected = still_there;
    assert!(protected.remove_finalizer("owner-finalizer-demo.kube.rs/cleanup"));
    patch_finalizers(&cms, &protected).await?;

    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if cms.get_opt("owner-finalizer-demo-protected").await?.is_none() {
            info!("Object was deleted once the finalizer was removed");
            break;
        }
        if tokio::time::Instant::now() > deadline {
            anyhow::bail!("timed out waiting for the object to be deleted after removing the finalizer");
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    info!("All checks passed");
    Ok(())
}
