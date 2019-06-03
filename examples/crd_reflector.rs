#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use kube::{
    api::{RawApi, Reflector, Void, Object},
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

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = RawApi::customResource("foos")
        .group("clux.dev")
        .within(&namespace);

    let rf : Reflector<Foo> = Reflector::raw(client, resource).init()?;

    loop {
        // Update internal state by calling watch (blocks):
        rf.poll()?;

        // Read updated internal state (instant):
        rf.read()?.into_iter().for_each(|(name, crd)| {
            info!("foo {}: {}", name, crd.spec.info);
        });
    }
}
