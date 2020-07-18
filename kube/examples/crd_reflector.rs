#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use kube::{
    api::{Api, ListParams, Meta},
    Client,
};
use kube_derive::CustomResource;
use kube_runtime::{reflector, utils::try_flatten_applied, watcher};
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug)]
#[kube(group = "clux.dev", version = "v1", namespaced)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let store = reflector::store::Writer::<Foo>::default();
    let reader = store.as_reader();
    let foos: Api<Foo> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(20); // low timeout in this example
    let rf = reflector(store, watcher(foos, lp));

    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let crds = reader.state().iter().map(Meta::name).collect::<Vec<_>>();
            info!("Current crds: {:?}", crds);
        }
    });
    let mut rfa = try_flatten_applied(rf).boxed_local();
    while let Some(event) = rfa.try_next().await? {
        info!("Applied {}", Meta::name(&event));
    }
    Ok(())
}
