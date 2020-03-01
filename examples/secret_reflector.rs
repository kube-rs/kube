#[macro_use] extern crate log;
use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{ListParams, Resource},
    client::APIClient,
    config,
    runtime::Reflector,
};

/// Example way to read secrets
#[derive(Debug)]
enum Decoded {
    /// Usually secrets are just short utf8 encoded strings
    Utf8(String),
    /// But it's allowed to just base64 encode binary in the values
    Bytes(Vec<u8>),
}


fn decode(secret: &Secret) -> BTreeMap<String, Decoded> {
    let mut res = BTreeMap::new();
    for (k, v) in secret.data.clone().unwrap() {
        if let Ok(b) = std::str::from_utf8(&v.0) {
            res.insert(k, Decoded::Utf8(b.to_string()));
        } else {
            res.insert(k, Decoded::Bytes(v.0));
        }
    }
    res
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let resource = Resource::namespaced::<Secret>(&namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf: Reflector<Secret> = Reflector::new(client, lp, resource).init().await?;

    // Can read initial state now:
    rf.state().await?.into_iter().for_each(|secret| {
        let res = decode(&secret);
        info!(
            "Found secret {} with data: {:?}",
            secret.metadata.unwrap().name.unwrap(),
            res
        );
    });

    loop {
        // Update internal state by calling watch (waits the full timeout)
        rf.poll().await?; // ideally call this from a thread/task

        // Read updated internal state (instant):
        let secrets = rf
            .state()
            .await?
            .into_iter()
            .map(|secret| secret.metadata.unwrap().name.unwrap())
            .collect::<Vec<_>>();
        info!("Current secrets: {:?}", secrets);
    }
}
