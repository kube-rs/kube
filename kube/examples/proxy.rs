#[macro_use] extern crate log;
use k8s_openapi::api::core::v1::Namespace;

use kube::{api::{Api, ListParams}, Client, Config};
use kube::config::KubeConfigOptions;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();

    let proxy_url = std::env::var("http_proxy").ok();
    if let Some(p) = &proxy_url {
        info!("http_proxy is {}", p);
    } else {
        warn!("You can set HTTP(s) proxy for this example with http_proxy environment variable");
    }

    let mut config = Config::from_kubeconfig(&KubeConfigOptions::default()).await?;
    let proxy  = proxy_url.map(|url| reqwest::Proxy::https(&url)).map_or(Ok(None), |p| p.map(Some))?;
    let config = proxy.map(|p| config.proxy(p)).unwrap_or(config);
    let client = Client::new(config);

    // Verify we can access kubernetes through proxy
    let ns_api: Api<Namespace> = Api::all(client);
    let namespaces = ns_api.list(&ListParams::default()).await?;
    assert!(namespaces.items.len() > 0);
    info!("Found {} namespaces", namespaces.items.len());

    Ok(())
}
