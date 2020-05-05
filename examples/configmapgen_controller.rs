use anyhow::Result;
use futures::{future, StreamExt};
use kube::{api::ListParams, Api, Client, Config};
use kube_derive::CustomResource;
use kube_rt::{
    controller::{controller, trigger_self, ReconcilerAction},
    reflector, try_flatten_addeds, watcher,
};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tokio::time::Duration;

#[derive(Debug, Snafu)]
enum Error {
    Borked,
}

#[derive(CustomResource, Debug, Clone, Deserialize, Serialize)]
#[kube(group = "nullable.se", version = "v1", namespaced)]
struct ConfigMapGeneratorSpec {
    content: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    println!(
        "{}",
        serde_json::to_string(&ConfigMapGenerator::crd()).unwrap()
    );

    let config = Config::infer().await?;
    let client = Client::new(config);
    let api = Api::<ConfigMapGenerator>::all(client);

    let cache = kube_rt::reflector::Cache::<ConfigMapGenerator>::default();
    controller(
        |generator| async move {
            println!("{:#?}", generator);
            Err(Error::Borked)
            // Ok(ReconcilerAction {
            //     requeue_after: Some(Duration::from_secs(2)),
            // })
        },
        |error| ReconcilerAction{requeue_after: Some(Duration::from_secs(1))},
        cache.clone(),
        trigger_self(try_flatten_addeds(reflector(
            cache,
            watcher(api, ListParams::default()),
        ))),
    )
    .for_each(|res| async move { println!("I did a thing! {:?}", res) })
    .await;

    Ok(())
}
