use color_eyre::Result;
use futures::prelude::*;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Client, Config, Tls, api::ListParams};
use kube_runtime::{reflector, watcher};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::infer(Tls::pick()).await?;
    let client = Client::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let api: Api<Pod> = Api::namespaced(client, &namespace);
    let store_w = reflector::store::Writer::default();
    let store = store_w.as_reader();
    let reflector = reflector(store_w, watcher(api, ListParams::default()));
    // Use try_for_each to fail on first error, use for_each to keep retrying
    reflector
        .try_for_each(|_event| async {
            println!("Current pod count: {}", store.state().len());
            Ok(())
        })
        .await?;
    Ok(())
}
