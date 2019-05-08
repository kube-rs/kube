extern crate failure;
extern crate kube;
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

    // rf is initialized with full state, which can be extracted on demand.
    // Output is Map of name -> (FooResource, _)
    rf.read()?.into_iter().for_each(|(name, (spec, _))| {
        println!("Found foo {}: {}", name, spec.info);
    });

    // r needs to have `r.poll()?` called continuosly to keep state up to date:
    loop {
        std::thread::sleep(std::time::Duration::from_secs(10));
        rf.poll()?;
        let deploys = rf.read()?.into_iter().map(|(name, _)| name).collect::<Vec<_>>();
        println!("Current foos: {:?}", deploys);
    }
}
