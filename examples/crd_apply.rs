#[macro_use]
extern crate log;
use futures::{StreamExt, TryStreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;

use kube::{
    api::{Api, ListParams, Meta, Patch, WatchEvent},
    Client, CustomResource,
};

// NB: This example uses server side apply and beta1 customresources
// Please test against Kubernetes 1.16.X!

// Own custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
#[kube(status = "FooStatus")]
#[kube(apiextensions = "v1beta1")] // remove this if using Kubernetes >= 1.17
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
pub struct FooSpec {
    name: String,
    info: Option<String>,
    replicas: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct FooStatus {
    is_bad: bool,
    replicas: i32,
}
fn make_patch<T>(obj: T) -> Patch<T> {
    Patch::Apply {
        patch: obj,
        field_manager: "crd_apply_example".to_string(),
        force: true,
    }
}
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // 0. Apply the CRD
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    match crds
        .patch("foos.clux.dev", &Default::default(), &make_patch(Foo::crd()))
        .await
    {
        Ok(o) => info!("Applied {}: ({:?})", Meta::name(&o), o.spec),
        Err(kube::Error::Api(ae)) => {
            warn!("apply error: {:?}", ae);
            assert_eq!(ae.code, 409); // if it's still there..
        }
        Err(e) => return Err(e.into()),
    }
    wait_for_crd_ready(&crds).await?; // wait for k8s to deal with it

    // Start applying foos
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    // 1. Apply from a full struct (e.g. equivalent to replace w/o resource_version)
    let foo = Foo::new(
        "baz",
        FooSpec {
            name: "baz".into(),
            info: Some("old baz".into()),
            replicas: 3,
        },
    );
    info!("Applying 1: \n{}", serde_yaml::to_string(&foo)?);
    let o = foos.patch("baz", &Default::default(), &make_patch(&foo)).await?;
    info!("Applied 1 {}: {:?}", Meta::name(&o), o.spec);

    // 2. Apply from partial json!
    // NB: requires TypeMeta + everything non-optional in the spec
    // NB: unfortunately optionals are nulled out by the apiserver...
    // (Because this does not go through K::Serialize it's not related to serde annots)
    // (it's actually defaulted by the server => crd schema needs to provide this info..)
    let patch = serde_json::json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "spec": {
            "name": "foo",
            "replicas": 2
        }
    });

    info!("Applying 2: \n{}", serde_yaml::to_string(&patch)?);
    let o2 = foos
        .patch("baz", &Default::default(), &make_patch(&patch))
        .await?;
    info!("Applied 2 {}: {:?}", Meta::name(&o2), o2.spec);

    Ok(())
}

async fn wait_for_crd_ready(crds: &Api<CustomResourceDefinition>) -> anyhow::Result<()> {
    if crds.get("foos.clux.dev").await.is_ok() {
        return Ok(());
    }
    // Wait for the apply to take place (takes a sec or two during first install)
    let lp = ListParams::default()
        .fields(&format!("metadata.name={}", "foos.clux.dev")) // our crd only
        .timeout(5); // should not take long
    let mut stream = crds.watch(&lp, "0").await?.boxed();

    while let Some(status) = stream.try_next().await? {
        if let WatchEvent::Modified(s) = status {
            info!("Modify event for {}", Meta::name(&s));
            if let Some(s) = s.status {
                if let Some(conds) = s.conditions {
                    if let Some(pcond) = conds.iter().find(|c| c.type_ == "NamesAccepted") {
                        if pcond.status == "True" {
                            info!("crd was accepted: {:?}", pcond);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    Err(anyhow::anyhow!("Timed out waiting for crd to become accepted"))
}
