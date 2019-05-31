#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use kube::{
    api::{RawApi, Reflector, Void},
    client::APIClient,
    config,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct Foo {
    name: String,
    info: String,
}

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = RawApi::customResource("foos")
        .group("clux.dev")
        .within("dev");

    let rf : Reflector<Foo, Void> = Reflector::raw(client, resource)
        .init()?;

    loop {
        // Update internal state by calling watch (blocks):
        rf.poll()?;

        // Read updated internal state (instant):
        rf.read()?.into_iter().for_each(|(name, crd)| {
            info!("foo {}: {}", name, crd.spec.info);
        });
    }
}
