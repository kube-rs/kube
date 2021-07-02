#[macro_use] extern crate log;
use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, LogParams},
    Client,
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "default".into());

    let mypod = env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("Usage: log_follow <pod>"))?;
    info!("Fetching logs for {:?} in {}", mypod, namespace);

    let pods: Api<Pod> = Api::namespaced(client, &namespace);
    let mut logs = pods
        .log_stream(&mypod, &LogParams {
            follow: true,
            tail_lines: Some(1),
            ..LogParams::default()
        })
        .await?
        .boxed();

    while let Some(line) = logs.try_next().await? {
        println!("{:?}", String::from_utf8_lossy(&line));
    }
    Ok(())
}
