#[macro_use] extern crate log;
#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;

use kube::{
    api::{Api, PostResponse, CreateResponse, PostParams, Object, Void},
    client::APIClient,
    config,
};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    //CustomResourceDefinition as Crd,
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionNames as CrdNames,
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
    let foocrd = CrdSpec {
        group: "clux.dev".into(),
        version: Some("v1".into()),
        scope: "Namespaced".into(),
        names: CrdNames {
            plural: "foos".into(),
            singular: Some("foo".into()),
            kind: "Foo".into(),
            ..Default::default()
        },
        ..Default::default()
    };
    let pp = PostParams::default();
    let req = crds.create(&pp, serde_json::to_vec(&foocrd)?)?;
    match client.request::<CreateResponse<Object<CrdSpec, CrdStatus>>>(req)? {
        CreateResponse::Created(o) => info!("Created {}", o.metadata.name),
        CreateResponse::Accepted(o) => info!("Accepted {}", o.metadata.name),
        CreateResponse::Ok(o) => info!("Ok {}", o.metadata.name),
        CreateResponse::Error => bail!("Uh oh"),
    }

    // Manage the Foo CR
    let foos = Api::customResource("foos.clux.dev").version("v1");

    // Create some Foos
    let f1 = FooSpec { name: "baz".into(), info: "unpatched baz".into() };
    foos.create(&pp, serde_json::to_vec(&f1)?)?;

    let f2 = FooSpec { name: "qux".into(), info: "unpatched qux".into() };
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
    match client.request::<PostResponse<Foo>>(req)? {
        PostResponse::Ok(o) => info!("Replaced status {:?} for {}", o.status, o.metadata.name),
        PostResponse::Created(o) => info!("Replaced status {:?} for {}", o.status, o.metadata.name),
        PostResponse::Error => bail!("uh oh 2"),
    }

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
