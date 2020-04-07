#[macro_use] extern crate log;
use kube_derive::CustomResource;
use serde::{Deserialize, Serialize};

use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
};

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
    let foos: Api<Foo> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(20); // low timeout in this example
    let rf = Reflector::new(foos).params(lp);
    let runner = rf.clone().run();

    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let crds = rf
                .state()
                .await
                .unwrap()
                .iter()
                .map(Meta::name)
                .collect::<Vec<_>>();
            info!("Current crds: {:?}", crds);
        }
    });
    runner.await?;
    Ok(())
}
