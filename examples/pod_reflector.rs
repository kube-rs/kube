use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, ListParams},
    runtime::{reflector, watcher, WatchStreamExt},
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
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info!("Current pod count: {}", reader.state().len());
            // full information with debug logs
            for p in reader.state() {
                let yaml = serde_yaml::to_string(p.as_ref()).unwrap();
                debug!("Pod {}: \n{}", p.name_any(), yaml);
            }
        }
    });

    let stream = watcher(api, ListParams::default()).map_ok(|ev| {
        ev.modify(|pod| {
            // memory optimization for our store - we don't care about fields/annotations/status
            pod.managed_fields_mut().clear();
            pod.annotations_mut().clear();
            pod.status = None;
        })
    });

    let rf = reflector(writer, stream).applied_objects();
    futures::pin_mut!(rf);

    while let Some(pod) = rf.try_next().await? {
        info!("saw {}", pod.name_any());
    }
    Ok(())
}
