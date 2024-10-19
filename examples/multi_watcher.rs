use futures::{stream, StreamExt, TryStreamExt};
use kube::{
    api::{Api, ResourceExt},
    k8s::{
        appsv1::Deployment,
        corev1::{ConfigMap, Secret},
    },
    runtime::{watcher, WatchStreamExt},
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = kube::Client::try_default().await?;

    let deploys: Api<Deployment> = Api::default_namespaced(client.clone());
    let cms: Api<ConfigMap> = Api::default_namespaced(client.clone());
    let secret: Api<Secret> = Api::default_namespaced(client.clone());
    let dep_watcher = watcher(deploys, watcher::Config::default());
    let cm_watcher = watcher(cms, watcher::Config::default());
    let sec_watcher = watcher(secret, watcher::Config::default());

    // select on applied events from all watchers
    let mut combo_stream = stream::select_all(vec![
        dep_watcher.applied_objects().map_ok(Watched::Deploy).boxed(),
        cm_watcher.applied_objects().map_ok(Watched::Config).boxed(),
        sec_watcher.applied_objects().map_ok(Watched::Secret).boxed(),
    ]);
    // SelectAll Stream elements must have the same Item, so all packed in this:
    #[allow(clippy::large_enum_variant)]
    enum Watched {
        Config(ConfigMap),
        Deploy(Deployment),
        Secret(Secret),
    }
    while let Some(o) = combo_stream.try_next().await? {
        match o {
            Watched::Config(cm) => info!("Got configmap: {}", cm.name_any()),
            Watched::Deploy(d) => info!("Got deployment: {}", d.name_any()),
            Watched::Secret(s) => info!("Got secret: {}", s.name_any()),
        }
    }
    Ok(())
}
