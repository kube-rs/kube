#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;
use std::time::Duration;
use futures_timer::Delay;

use kube::{
    api::{Object, RawApi, Reflector, Void},
    client::APIClient,
    config,
};

// Own custom resource spec
#[derive(Deserialize, Serialize, Clone)]
pub struct FooSpec {
    name: String,
    info: String,
}
// The kubernetes generic object with our spec and no status
type Foo = Object<FooSpec, Void>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = RawApi::customResource("foos")
        .group("clux.dev")
        .within(&namespace);

    let rf : Reflector<Foo> = Reflector::raw(client, resource)
        .timeout(20) // low timeout in this example
        .init().await?;

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
        let crds = rf.state().await?.into_iter().map(|crd| crd.metadata.name).collect::<Vec<_>>();
        info!("Current crds: {:?}", crds);
    }
}
