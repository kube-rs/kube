use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Namespace;
use kube::{
    api::Api,
    runtime::{predicates, reflector, watcher, WatchStreamExt},
    Client, ResourceExt,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let api: Api<Namespace> = Api::all(client);
    let (reader, writer) = reflector::store::<Namespace>();

    tokio::spawn(async move {
        // Show state every 5 seconds of watching
        loop {
            reader.wait_until_ready().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info!("Current namespace count: {}", reader.state().len());
            // full information with debug logs
            for p in reader.state() {
                let yaml = serde_yaml::to_string(p.as_ref()).unwrap();
                debug!("Namespace {}: \n{}", p.name_any(), yaml);
            }
        }
    });

    let stream = watcher(api, watcher::Config::default().streaming_lists())
        .default_backoff()
        .modify(|ns| {
            // memory optimization for our store - we don't care about managed fields/annotations/status
            ns.managed_fields_mut().clear();
            ns.annotations_mut().clear();
            ns.status = None;
        })
        .reflect(writer)
        .applied_objects()
        .predicate_filter(predicates::resource_version); // NB: requires an unstable feature

    futures::pin_mut!(stream);
    while let Some(ns) = stream.try_next().await? {
        info!("saw {}", ns.name_any());
    }
    Ok(())
}
