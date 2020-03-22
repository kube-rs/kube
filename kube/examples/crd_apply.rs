#[macro_use] extern crate log;
use futures_timer::Delay;
use kube_derive::CustomResource;
use serde_derive::{Deserialize, Serialize};
use serde_yaml;
use std::time::Duration;

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;

use kube::{
    api::{Api, Meta, PatchParams, PatchStrategy},
    client::APIClient,
    config,
};

// NB: This example uses server side apply and beta1 customresources
// Please test against kubernetes 1.16.X!

// Own custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
#[kube(status = "FooStatus")]
#[kube(apiextensions = "v1beta1")] // remove this if using kubernetes >= 1.17
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
pub struct FooSpec {
    name: String,
    info: Option<String>,
    replicas: i32,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FooStatus {
    is_bad: bool,
    replicas: i32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=info");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let ssapply = PatchParams {
        patch_strategy: PatchStrategy::Apply,
        // always override on conflicts
        force: true,
        // owner of the fields: (us)
        field_manager: Some("crd_apply_example".to_string()),
        ..Default::default()
    };

    // 0. Install the CRD
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    match crds
        .patch("foos.clux.dev", &ssapply, serde_yaml::to_vec(&Foo::crd())?)
        .await
    {
        Ok(o) => info!("Applied {}: ({:?})", Meta::name(&o), o.spec),
        Err(kube::Error::Api(ae)) => {
            warn!("apply error: {:?}", ae);
            assert_eq!(ae.code, 409); // if it's still there..
        }
        Err(e) => return Err(e.into()),
    }
    // Wait for the apply to take place
    Delay::new(Duration::from_secs(2)).await;

    // Start applying foos
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    // 1. Apply from a full struct (e.g. equivalent to replace w/o resource_version)
    let foo = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: Some("old baz".into()),
        replicas: 3,
    });
    info!("Applying 1: \n{}", serde_yaml::to_string(&foo)?);
    let o = foos.patch("baz", &ssapply, serde_yaml::to_vec(&foo)?).await?;
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
    let o2 = foos.patch("baz", &ssapply, serde_yaml::to_vec(&patch)?).await?;
    info!("Applied 2 {}: {:?}", Meta::name(&o2), o2.spec);

    /*    // 3. apply from partial yaml (EXPERIMENT, IGNORE, VERY BAD)
        let yamlpatch2 = r#"
            spec:
                info: "newer baz"
                name: "foo"
        "#;
        let o3 = foos.apply("baz", yamlpatch2).await?;
        assert_eq!(o3.spec.info, "newer baz");
        info!("Applied 3 {}: {:?}", Meta::name(&o3), o3.spec);
    */
    Ok(())
}
