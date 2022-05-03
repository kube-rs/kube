use anyhow::{anyhow, Result};
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, LogParams},
    Client,
};
use std::env;
use tracing::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
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
        info!("{:?}", String::from_utf8_lossy(&line));
    }
    Ok(())
}
