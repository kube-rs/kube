#[macro_use] extern crate log;
use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};

/// Example way to read secrets
fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1ConfigMap(client).within(&namespace);
    let rf = Reflector::new(resource).init()?;

    // Can read initial state now:
    rf.read()?.into_iter().for_each(|config_map| {
        info!("Found configmap {} with data: {:?}", config_map.metadata.name, config_map.data);
    });

    // Poll to keep data up to date:
    loop {
        rf.poll()?;

        // up to date state:
        let pods = rf.read()?.into_iter()
            .map(|config_map| config_map.metadata.name)
            .collect::<Vec<_>>();

        info!("Current configmaps: {:?}", pods);
    }
}
