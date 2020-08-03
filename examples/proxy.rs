#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Namespace;

use kube::{
    api::{Api, ListParams},
    config::KubeConfigOptions,
    Client, Config,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();

    let mut config = Config::from_kubeconfig(&KubeConfigOptions::default()).await?;

    if let Ok(proxy_url) = &std::env::var("PROXY_URL") {
        info!("PROXY_URL is {}", proxy_url);
        config = config.proxy(reqwest::Proxy::https(proxy_url)?);
    } else {
        warn!("Running without PROXY_URL environment variable set");
    }

    let client = Client::new(config);

    // Verify we can access kubernetes through proxy
    let ns_api: Api<Namespace> = Api::all(client);
    let namespaces = ns_api.list(&ListParams::default()).await?;
    assert!(namespaces.items.len() > 0);
    info!("Found {} namespaces", namespaces.items.len());

    Ok(())
}
