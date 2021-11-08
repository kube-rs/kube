use color_eyre::Result;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams, ResourceExt},
    runtime::cache::Cache,
    Client,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::try_default().await?;
    let api: Api<Pod> = Api::default_namespaced(client);
    let cache = Cache::new(api, ListParams::default());
    let store = cache.store();

    // Observe kubernetes watch events while driving the cache:
    let mut applies = cache.applies();
    while let Some(p) = applies.try_next().await? {
        println!("Got pod: {} (total={})", p.name(), store.state().len());
    }

    Ok(())
}
