#[macro_use] extern crate log;
use std::collections::BTreeMap;

use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, ListParams, Meta},
    runtime::Reflector,
    Client,
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
    // Ignoring binary data for now
    if let Some(data) = secret.data.clone() {
        for (k, v) in data {
            if let Ok(b) = std::str::from_utf8(&v.0) {
                res.insert(k, Decoded::Utf8(b.to_string()));
            } else {
                res.insert(k, Decoded::Bytes(v.0));
            }
        }
    }
    res
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let secrets: Api<Secret> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example
    let rf = Reflector::new(secrets).params(lp);

    let rf2 = rf.clone(); // read from a clone in a task
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            tokio::time::delay_for(std::time::Duration::from_secs(5)).await;
            let secrets: Vec<_> = rf2
                .state()
                .await
                .unwrap()
                .iter()
                .map(|s| format!("{}: {:?}", Meta::name(s), decode(s).keys()))
                .collect();
            info!("Current secrets: {:?}", secrets);
        }
    });

    rf.run().await?; // run reflector and listen for signals
    Ok(())
}
