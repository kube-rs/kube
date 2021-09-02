#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiexts;

use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt, WatchEvent},
    Client, CustomResource, CustomResourceExt,
};

// NB: This example uses server side apply and beta1 customresources
// Please test against Kubernetes 1.16.X!

// Own custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
#[kube(status = "FooStatus")]
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
pub struct FooSpec {
    name: String,
    info: Option<String>,
    replicas: isize,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct FooStatus {
    is_bad: bool,
    replicas: isize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let ssapply = PatchParams::apply("crd_apply_example").force();

    // 0. Ensure the CRD is installed (you probably just want to do this on CI)
    // (crd file can be created by piping `Foo::crd`'s yaml ser to kubectl apply)
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    crds.patch("foos.clux.dev", &ssapply, &Patch::Apply(Foo::crd()))
        .await?;
    wait_for_crd_ready(&crds).await?; // wait for k8s to deal with it

    // Start applying foos
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    // 1. Apply from a full struct (e.g. equivalent to replace w/o resource_version)
    let foo = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: Some("old baz".into()),
        replicas: 3,
    });
    info!("Applying 1: \n{}", serde_yaml::to_string(&foo)?);
    let o = foos.patch("baz", &ssapply, &Patch::Apply(&foo)).await?;
    // NB: kubernetes < 1.20 will fail to admit scale subresources - see #387
    info!("Applied 1 {}: {:?}", o.name(), o.spec);

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
    info!("Applied 2 {}: {:?}", o2.name(), o2.spec);

    Ok(())
}

// manual way to check that a CRD has been installed
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
            info!("Modify event for {}", s.name());
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
