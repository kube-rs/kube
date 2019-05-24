#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;
use serde_json::json;

use kube::{
    api::{Api, PostResponse, PostParams, Object},
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct FooStatus {
    is_bad: bool,
}

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // Manage the CRD
    let crds = Api::v1beta1CustomResourceDefinition();

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
            }
        }
    });

    let pp = PostParams::default();
    let req = crds.create(&pp, serde_json::to_vec(&foocrd)?)?;
    if let Ok(res) = client.request::<Object<CrdSpec, CrdStatus>>(req) {
        info!("Created {}", res.metadata.name);
        debug!("Created CRD: {:?}", res.spec);
    } else {
        // TODO: need error code here for ease
    }


    // Manage the Foo CR
    let foos = Api::customResource("foos.clux.dev").version("v1");

    // Create some Foos
    let f1 = json!({
        "metadata": { "name": "baz" },
        "spec": FooSpec { name: "baz".into(), info: "unpatched baz".into() }
    });
    foos.create(&pp, serde_json::to_vec(&f1)?)?;

    let f2 = json!({
        "metadata": { "name": "qux" },
        "spec": FooSpec { name: "qux".into(), info: "unpatched qux".into() }
    });
    foos.create(&pp, serde_json::to_vec(&f2)?)?;


    // Modify a Foo with a Patch
    //let patch = json!( info => "patched baz" );
    //let req = foos.patch("baz", &pp, serde_json::to_vec(&patch)?)?;
    //client.request::<PostResponse<Object<FooSpec, FooStatus>>>(req)?;

    // shorthand
    type Foo = Object<FooSpec, FooStatus>;
    // TODO: request should return statuscode as a better useability!

    // Set its status:
    let fs = FooStatus { is_bad: true };
    let req = foos.replace_status("baz", &pp, serde_json::to_vec(&fs)?)?;
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
