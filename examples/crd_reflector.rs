#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate k8s_openapi_derive;
use futures_timer::Delay;
use std::time::Duration;

use kube::{
    api::{CustomResource, ListParams},
    client::APIClient,
    config,
    runtime::Reflector,
};

#[derive(CustomResourceDefinition, Deserialize, Serialize, Clone, Debug, PartialEq)]
#[custom_resource_definition(group = "clux.dev", version = "v1", plural = "foos", namespaced)]
pub struct FooSpec {
    name: String,
    info: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = CustomResource::kind("Foo")
        .group("clux.dev")
        .version("v1")
        .within(&namespace)
        .into_resource();

    let lp = ListParams::default().timeout(20); // low timeout in this example

    let rf: Reflector<Foo> = Reflector::new(client, lp, resource).init().await?;

    let cloned = rf.clone();
    tokio::spawn(async move {
        loop {
            if let Err(e) = cloned.poll().await {
                warn!("Poll error: {:?}", e);
            }
        }
    });

    loop {
        Delay::new(Duration::from_secs(5)).await;
        // Read updated internal state (instant):
        let crds = rf
            .state()
            .await?
            .into_iter()
            .map(|crd| crd.metadata.unwrap().name.unwrap())
            .collect::<Vec<_>>();
        info!("Current crds: {:?}", crds);
    }
}
