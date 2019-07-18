#[macro_use]
extern crate log;

use kube::{
    api::{Api, Log, LogParams},
    client::APIClient,
    config,
};
use std::env;

fn main() -> Result<(), failure::Error> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let config = config::load_kube_config().expect("failed to load kubeconfig");
    let client = APIClient::new(config);

    let mypod = match env::args().nth(1) {
        Some(pod) => pod,
        None => {
            println!("Usage: log_openapi <pod>");
            return Ok(());
        }
    };

    // Manage pods
    let pods = Api::v1Pod(client);;
    let lp = LogParams::default();
    let plog = pods.within("default").log(&mypod, &lp)?;
    println!("Got pod {} log: {}", &mypod, &plog);

    Ok(())
}
