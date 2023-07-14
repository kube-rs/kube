use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
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

    let api: Api<Pod> = Api::default_namespaced(client);
    let (reader, writer) = reflector::store::<Pod>();

    tokio::spawn(async move {
        // Show state every 5 seconds of watching
        loop {
            reader.wait_until_ready().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info!("Current pod count: {}", reader.state().len());
            // full information with debug logs
            for p in reader.state() {
                let yaml = serde_yaml::to_string(p.as_ref()).unwrap();
                debug!("Pod {}: \n{}", p.name_any(), yaml);
            }
        }
    });

    let stream = watcher(api, watcher::Config::default().any_semantic())
        .default_backoff()
        .modify(|pod| {
            // memory optimization for our store - we don't care about managed fields/annotations/status
            pod.managed_fields_mut().clear();
            pod.annotations_mut().clear();
            pod.status = None;
        })
        .reflect(writer)
        .applied_objects()
        .predicate_filter(predicates::resource_version); // NB: requires an unstable feature
    futures::pin_mut!(stream);

    while let Some(pod) = stream.try_next().await? {
        info!("saw {}", pod.name_any());
    }
    Ok(())
}
