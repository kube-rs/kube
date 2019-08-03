#[macro_use] extern crate log;
use std::collections::BTreeMap;

use kube::{
    api::{Api, Reflector},
    client::APIClient,
    config,
};

/// Example way to read secrets
#[derive(Debug)]
enum Decoded {
    /// Usually secrets are just short utf8 encoded strings
    Utf8(String),
    /// But it's allowed to just base64 encode binary in the values
    Bytes(Vec<u8>),
}

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Api::v1Secret(client).within(&namespace);
    let rf = Reflector::new(resource).init()?;

    // Can read initial state now:
    rf.read()?.into_iter().for_each(|secret| {
        let mut res = BTreeMap::new();
        for (k, v) in secret.data {
            if let Ok(b) = std::str::from_utf8(&v.0) {
                res.insert(k, Decoded::Utf8(b.to_string()));
            }
            else {
                res.insert(k, Decoded::Bytes(v.0));
            }
        }
        info!("Found secret {} with data: {:?}",
            secret.metadata.name,
            res,
        );
    });

    // Poll to keep data up to date:
    loop {
        rf.poll()?;

        // up to date state:
        let pods = rf.read()?.into_iter().map(|secret| secret.metadata.name).collect::<Vec<_>>();
        info!("Current pods: {:?}", pods);
    }
}
