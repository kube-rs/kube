#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

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
        .timeout(10) // low timeout in this example
        .init().await?;

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?; // ideally call this from a thread/task

        // Read updated internal state (instant):
        rf.state().await?.into_iter().for_each(|crd| {
            info!("foo {}: {}", crd.metadata.name, crd.spec.info);
        });
    }
}
