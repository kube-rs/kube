#[macro_use] extern crate log;
use futures::StreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::cache::{Cache, Store},
    Client,
};
use std::collections::BTreeMap;

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

fn spawn_periodic_reader(reader: Store<Secret>) {
    tokio::spawn(async move {
        loop {
            // Periodically read our state
            let cms: Vec<_> = reader
                .state()
                .iter()
                .map(|s| format!("{}: {:?}", s.name(), decode(s).keys()))
                .collect();
            info!("Current secrets: {:?}", cms);
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let secrets: Api<Secret> = Api::default_namespaced(client);

    let cache = Cache::new(secrets, ListParams::default());
    spawn_periodic_reader(cache.store()); // read from a reader in the background

    let stream = cache.applies();
    stream
        .for_each(|s| async move {
            log::info!("Saw: {}", s.name());
        })
        .await;
    Ok(())
}
