#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use kube::{
    api::{ApiResource, ReflectorSpec as Reflector},
    client::APIClient,
    config,
};

// Own custom resource
#[derive(Deserialize, Serialize, Clone)]
pub struct FooResource {
  name: String,
  info: String,
}

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    // This example requires `kubectl apply -f examples/foo.yaml` run first
    let resource = ApiResource {
        group: "clux.dev".into(),
        resource: "foos".into(),
        version: "v1".into(),
        namespace: Some("kube-system".into()),
        ..Default::default()
    };
    let rf : Reflector<FooResource> = Reflector::new(client, resource)?;

    loop {
        // Update internal state by calling watch (blocks):
        rf.poll()?;

        // Read updated internal state (instant):
        rf.read()?.into_iter().for_each(|(name, crd)| {
            info!("foo {}: {}", name, crd.spec.info);
        });

        std::thread::sleep(std::time::Duration::from_secs(10));
    }
}
