#[macro_use] extern crate log;
use kube::{
    api::{Api, LogParams},
    client::APIClient,
    config,
};
use anyhow::{Result, anyhow};
use std::env;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().await?;
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let mypod = env::args().nth(1).ok_or_else(|| {
        anyhow!("Usage: log_follow <pod>")
    })?;

    info!("My pod is {:?}", mypod);

    let pods = Api::v1Pod(client).within(&namespace);
    let mut lp = LogParams::default();
    lp.tail_lines = Some(1);
    let mut logs = pods.log_follow(&mypod, &lp).await?.boxed();

    while let Some(line) = logs.next().await {
        let l = line.unwrap();
        println!("{:?}", String::from_utf8_lossy(&l));
    }
    Ok(())
}
