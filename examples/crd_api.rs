#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use kube::{
    api::{Api, PostResponse, Resource},
    client::APIClient,
    config,
};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionStatus as CrdStatus,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct FooSpec {
    name: String,
    info: String,
}

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let crds = Api::v1beta1CustomResourceDefinition();
    let req = crds.create()?;

    // TODO: need full Resource<Foo, Void> of name "foo.clux.dev"
    let res = client.request::<PostResponse<Resource<CrdSpec, CrdStatus>>>(req)?;
    info!("create crd: {}", serde_json::to_string(&res)?);
    Ok(())
}
