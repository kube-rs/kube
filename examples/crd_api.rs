#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;
use serde_json::json;

use kube::{
    api::{Api, PostResponse, PostParams, DeleteParams, Object, PropagationPolicy, Void},
    client::APIClient,
    config,
};

//use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    //CustomResourceDefinition as Crd,
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionStatus as CrdStatus,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FooStatus {
    is_bad: bool,
}

// shorthands
type Foo = Object<FooSpec, FooStatus>;
type FooMeta = Object<Void, Void>;
type FullCrd = Object<CrdSpec, CrdStatus>;

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // Manage the CRD
    let crds = Api::v1beta1CustomResourceDefinition();

    // Delete any old versions of it first:
    let dp = DeleteParams {
        propagation_policy: Some(PropagationPolicy::Foreground),
        ..Default::default()
    };
    //let req = crds.delete("foos.clux.dev", &dp)?;
    //if let Ok(res) = client.request::<FullCrd>(req) {
    //    info!("Deleted {}: ({:?})", res.metadata.name, res.status.conditions.unwrap().last());
    //    use std::{thread, time};
    //    let five_secs = time::Duration::from_millis(5000);
    //    thread::sleep(five_secs);
    //    // even foreground policy doesn't seem to block here..
    //}

    // Create the CRD so we can create Foos in kube
    let foocrd = json!({
        "metadata": {
            "name": "foos.clux.dev"
        },
        "spec": {
            "group": "clux.dev",
            "version": "v1",
            "scope": "Namespaced",
            "names": {
                "plural": "foos",
                "singular": "foo",
                "kind": "Foo",
            },
            "subresources": {
                "status": {}
            },
        },
    });

    info!("Creating CRD foos.clux.dev");
    let pp = PostParams::default();
    let req = crds.create(&pp, serde_json::to_vec(&foocrd)?)?;
    if let Ok(res) = client.request::<FullCrd>(req) {
        info!("Created {}", res.metadata.name);
        debug!("Created CRD: {:?}", res.spec);
    } else {
        // TODO: need error code here for ease - 409 common
    }

    // Manage the Foo CR
    let foos = Api::customResource("foos")
        .version("v1")
        .group("clux.dev")
        .within("dev");

    // Create Foo baz
    info!("Creating Foo instance baz");
    let f1 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "baz", "info": "old" },
    });
    let req = foos.create(&pp, serde_json::to_vec(&f1)?)?;
    let o = client.request::<FooMeta>(req)?;
    info!("Created {}", o.metadata.name);

    // Modify a Foo baz with a Patch
    info!("Patch Foo instance baz");
    let patch = json!({
        "spec": { "info": "patched baz" }
    });
    let req = foos.patch("baz", &pp, serde_json::to_vec(&patch)?)?;
    let o = client.request::<Foo>(req)?;
    info!("Patched {} with new name: {}", o.metadata.name, o.spec.name);

    info!("Create Foo instance qux");
    let f2 = json!({
        "apiVersion": "clux.dev/v1",
        "kind": "Foo",
        "metadata": { "name": "qux" },
        "spec": FooSpec { name: "qux".into(), info: "unpatched qux".into() },
        "status": FooStatus::default(),
    });
    let req = foos.create(&pp, serde_json::to_vec(&f2)?)?;
    let o = client.request::<FooMeta>(req)?;
    info!("Created {}", o.metadata.name);


    // Update status on -qux
    info!("Replace Status on Foo instance qux");
    let fs = json!({
        "status": FooStatus { is_bad: true }
    });
    let req = foos.replace_status("qux", &pp, serde_json::to_vec(&fs)?)?;
    let res = client.request::<Foo>(req)?;
    info!("Replaced status {:?} for {}", res.status, res.metadata.name);


    // Verify we can get it
    let req = foos.get("baz")?;
    let f2cpy = client.request::<Foo>(req)?;
    assert_eq!(f2cpy.spec.info, "patched baz");

    // The other one has no status:
    let req = foos.get("qux")?;
    let f2 = client.request::<Foo>(req)?;
    assert_eq!(f2.spec.info, "unpatched qux");

    Ok(())
}
