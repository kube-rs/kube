#[macro_use] extern crate log;
use kube_derive::CustomResource;
use serde_derive::{Deserialize, Serialize};
use serde_yaml;

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
//#[kube(scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#)]
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
    //let jsonmerge = PatchParams::default();

    // 0. Install the CRD
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    match crds.patch("foos.clux.dev", &ssapply, serde_yaml::to_vec(&Foo::crd())?).await {
        Ok(o) => info!("Applied {}: ({:?})", Meta::name(&o), o.spec),
        Err(kube::Error::Api(ae)) => {
            warn!("apply error: {:?}", ae);
            assert_eq!(ae.code, 409); // if you skipped delete, for instance
        },
        Err(e) => return Err(e.into()),
    }

    // 1. Create a Foo
    let foos: Api<Foo> = Api::namespaced(client.clone(), &namespace);

    let f1 = Foo::new("baz", FooSpec {
        name: "baz".into(),
        info: "old baz".into(),
        replicas: 3,
    });
    info!("Apply: \n{}", serde_yaml::to_string(&f1)?);

    // 1a). Invalid yaml apply (due to ints being made into floats)
    let o = foos.patch("baz", &ssapply, serde_yaml::to_vec(&f1)?).await?;

    // 1b). Valid json Merge strategy (somehow validates)
    //let o = foos.patch("baz", &jsonmerge, serde_json::to_vec(&f1)?).await?;

    info!("Created {}: {:?}", Meta::name(&o), o.spec);

    /*// Apply a subset of baz
    let patch = r#"
        spec:
            info: "new baz"
            name: "foo"
    "#;
    info!("Apply: {:?}", serde_yaml::from_str(&patch)?);
    let o = foos.patch("baz", &pp, serde_yaml::to_vec(patch)?).await?;
    info!("Applied {}: {:?}", Meta::name(&o), o.spec);*/

    Ok(())
}
