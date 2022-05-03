use futures::{StreamExt, TryStreamExt};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use tracing::*;

use kube::{
    api::{Api, ListParams, Patch, PatchParams, ResourceExt},
    runtime::{reflector, utils::try_flatten_applied, watcher},
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
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    // 0. Ensure the CRD is installed (you probably just want to do this on CI)
    // (crd file can be created by piping `Foo::crd`'s yaml ser to kubectl apply)
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&Foo::crd())?);
    let ssapply = PatchParams::apply("crd_reflector_example").force();
    crds.patch("foos.clux.dev", &ssapply, &Patch::Apply(Foo::crd()))
        .await?;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await; // wait for k8s to deal with it

    // 1. Run a reflector against the installed CRD
    let store = reflector::store::Writer::<Foo>::default();
    let reader = store.as_reader();
    let foos: Api<Foo> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(20); // low timeout in this example
    let rf = reflector(store, watcher(foos, lp));

    tokio::spawn(async move {
        loop {
            // Periodically read our state
            // while this runs you can kubectl apply -f crd-baz.yaml or crd-qux.yaml and see it works
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            let crds = reader.state().iter().map(|r| r.name()).collect::<Vec<_>>();
            info!("Current crds: {:?}", crds);
        }
    });
    let mut rfa = try_flatten_applied(rf).boxed();
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {}", event.name());
    }
    Ok(())
}
