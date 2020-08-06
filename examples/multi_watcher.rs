#[macro_use] extern crate log;
use futures::{stream, StreamExt, TryStreamExt};
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Secret},
};
use kube::{
    api::{Api, ListParams, Meta},
    Client,
};
use kube_runtime::{utils::try_flatten_applied, watcher};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,node_watcher=debug,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let namespace = std::env::var("NAMESPACE").unwrap_or("default".into());

    let deploys: Api<Deployment> = Api::namespaced(client.clone(), &namespace);
    let cms: Api<ConfigMap> = Api::namespaced(client.clone(), &namespace);
    let secret: Api<Secret> = Api::namespaced(client.clone(), &namespace);
    let dep_watcher = watcher(deploys, ListParams::default());
    let cm_watcher = watcher(cms, ListParams::default());
    let sec_watcher = watcher(secret, ListParams::default());

    // select on applied events from all watchers
    enum Watched {
        Config(ConfigMap),
        Deploy(Deployment),
        Secret(Secret),
    }
    let mut combo_stream = stream::select_all(vec![
        try_flatten_applied(dep_watcher).map_ok(Watched::Deploy).boxed(),
        try_flatten_applied(cm_watcher).map_ok(Watched::Config).boxed(),
        try_flatten_applied(sec_watcher).map_ok(Watched::Secret).boxed(),
    ]);
    while let Some(o) = combo_stream.try_next().await? {
        match o {
            Watched::Config(cm) => info!("Got configmap: {}", Meta::name(&cm)),
            Watched::Deploy(d) => info!("Got deployment: {}", Meta::name(&d)),
            Watched::Secret(s) => info!("Got secret: {}", Meta::name(&s)),
        }
    }
    Ok(())
}
