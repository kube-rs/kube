use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::{reflector, reflector::Store, watcher, WatchStreamExt},
    Client,
};
use std::collections::BTreeMap;
use tracing::*;

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
                .map(|s| format!("{}: {:?}", s.name_any(), decode(s).keys()))
                .collect();
            info!("Current secrets: {:?}", cms);
            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
        }
    });
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let secrets: Api<Secret> = Api::default_namespaced(client);
    let lp = ListParams::default().timeout(10); // short watch timeout in this example

    let (reader, writer) = reflector::store::<Secret>();
    let rf = reflector(writer, watcher(secrets, lp));

    spawn_periodic_reader(reader); // read from a reader in the background

    rf.applied_objects()
        .try_for_each(|s| async move {
            info!("saw: {}", s.name_any());
            Ok(())
        })
        .await?;
    Ok(())
}
