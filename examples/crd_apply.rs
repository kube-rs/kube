//! Generated types support documentation
#![deny(missing_docs)]
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::*;

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiexts;

use kube::{
    api::{Api, Patch, PatchParams, ResourceExt},
    runtime::wait::{await_condition, conditions},
    Client, CustomResource, CustomResourceExt,
};

/// Spec object for Foo
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
#[kube(status = "FooStatus")]
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
pub struct FooSpec {
    name: String,
    info: Option<String>,
    replicas: isize,
}

/// Status object for Foo
#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct FooStatus {
    is_bad: bool,
    replicas: isize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let ssapply = PatchParams::apply("crd_apply_example").force();

    // 0. Ensure the CRD is installed (you probably just want to do this on CI)
    // (crd file can be created by piping `Foo::crd`'s yaml ser to kubectl apply)
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    crds.patch("foos.clux.dev", &ssapply, &Patch::Apply(Foo::crd()))
        .await?;

    info!("Waiting for the api-server to accept the CRD");
    let establish = await_condition(crds, "foos.clux.dev", conditions::is_crd_established());
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), establish).await?;

    // Start applying foos
    let foos: Api<Foo> = Api::default_namespaced(client.clone());

    // 1. Apply from a full struct (e.g. equivalent to replace w/o resource_version)
    let foo = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: Some("old baz".into()),
        replicas: 3,
    });
    info!("Applying 1: \n{}", serde_yaml::to_string(&foo)?);
    let o = foos.patch("baz", &ssapply, &Patch::Apply(&foo)).await?;
    // NB: kubernetes < 1.20 will fail to admit scale subresources - see #387
    info!("Applied 1 {}: {:?}", o.name_any(), o.spec);

    // 2. Apply from partial json!
    let patch = serde_json::json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "spec": {
            "name": "foo",
            "replicas": 2
        }
    });

    info!("Applying 2: \n{}", serde_yaml::to_string(&patch)?);
    let o2 = foos.patch("baz", &ssapply, &Patch::Apply(patch)).await?;
    info!("Applied 2 {}: {:?}", o2.name_any(), o2.spec);

    Ok(())
}
