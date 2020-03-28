#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate kube_derive;
use futures_timer::Delay;
use std::time::Duration;

use kube::{
    api::{ListParams, Meta, Resource},
    Client,
    runtime::Reflector,
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
    let client = Client::new().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = Resource::namespaced::<Foo>(&namespace);
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
        let crds = rf.state().await?.iter().map(Meta::name).collect::<Vec<_>>();
        info!("Current crds: {:?}", crds);
    }
}
