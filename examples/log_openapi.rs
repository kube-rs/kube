#[macro_use] extern crate log;
use failure::{err_msg, Error};
use kube::{
    api::{Api, Log, LogParams},
    client::APIClient,
    config,
};
use std::env;

fn main() -> Result<(), Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let mypod = env::args().nth(1).ok_or_else(|| {
        err_msg("Usage: log_openapi <pod>")
    })?;

    // Get the logs from the specified pod
    // because we don't specify lp.container the pod must have only 1 container
    let pods = Api::v1Pod(client).within(&namespace);
    let lp = LogParams::default();
    let plog = pods.log(&mypod, &lp)?;
    info!("Got pod {} log: {}", &mypod, &plog);

    Ok(())
}
