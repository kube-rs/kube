#[macro_use] extern crate log;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::ConfigMap;
use kube::{
    api::{Api, ListParams},
    runtime::Informer,
    Client,
};

/// Example way to read secrets
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=trace");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let cms: Api<ConfigMap> = Api::namespaced(client, &namespace);
    let lp = ListParams::default().allow_bookmarks().timeout(10); // short watch timeout in this example
    let inf = Informer::new(cms).params(lp);

    loop {
        let mut stream = inf.poll().await?.boxed();
        while let Some(event) = stream.try_next().await? {
            info!("Got: {:?}", event);
        }
    }
}
