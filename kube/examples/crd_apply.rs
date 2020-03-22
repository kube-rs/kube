#[macro_use] extern crate log;
use kube_derive::CustomResource;
use serde_derive::{Deserialize, Serialize};
use serde_yaml;
use futures_timer::Delay;
use std::time::Duration;

use apiexts::CustomResourceDefinition;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;

use kube::{
    api::{Api, Meta, PatchParams, PatchStrategy},
    client::APIClient,
    config,
};

// Own custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
#[kube(status = "FooStatus")]
#[kube(apiextensions = "v1beta1")]
#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
pub struct FooSpec {
    name: String,
    info: String,
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
    match crds.patch("foos.clux.dev", &ssapply, serde_yaml::to_vec(&Foo::crd())?).await {
        Ok(o) => info!("Applied {}: ({:?})", Meta::name(&o), o.spec),
        Err(kube::Error::Api(ae)) => {
            warn!("apply error: {:?}", ae);
            assert_eq!(ae.code, 409); // if it's still there..
        },
        Err(e) => return Err(e.into()),
    }
    // Wait for the apply to take place
    Delay::new(Duration::from_secs(2)).await;

    // 1. Create a Foo
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    let foo = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: "old baz".into(),
        replicas: 3,
    });
    info!("Applying: \n{}", serde_yaml::to_string(&foo)?);
    let o = foos.patch("baz", &ssapply, serde_yaml::to_vec(&foo)?).await?;
    info!("Applied {}: {:?}", Meta::name(&o), o.spec);

    // Apply from rawstring yaml:
    let yamlpatch = r#"
        spec:
            info: "new baz"
            name: "foo"
    "#;
    info!("Apply: {:?}", yamlpatch);
    let o2 = foos.patch("baz", &ssapply, serde_yaml::to_vec(yamlpatch)?).await?;
    info!("Applied {}: {:?}", Meta::name(&o2), o2.spec);

    Ok(())
}
