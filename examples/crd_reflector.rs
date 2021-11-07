#[macro_use] extern crate log;
use futures::StreamExt;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;

use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    runtime::cache::Cache,
    Client, CustomResource, CustomResourceExt,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    // 0. Ensure the CRD is installed (you probably just want to do this on CI)
    // (crd file can be created by piping `Foo::crd`'s yaml ser to kubectl apply)
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    let ssapply = PatchParams::apply("crd_reflector_example").force();
    crds.patch("foos.clux.dev", &ssapply, &Patch::Apply(Foo::crd()))
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await; // wait for k8s to deal with it

    // 1. Run a reflector against the installed CRD
    let foos: Api<Foo> = Api::default_namespaced(client);
    let cache = Cache::new(foos, ListParams::default());
    let store = cache.store();

    // Observe kubernetes watch events while driving the cache:
    let mut applies = cache.applies().boxed();
    while let Some(foo) = applies.next().await {
        info!("Saw Foo: {} (total={})", foo.name(), store.state().len());
    }
    Ok(())
}
