//! In this example we will implement something similar
//! to `kubectl get all --all-namespaces`.

use kube::{
    api::{Api, DynamicObject, ResourceExt},
    client::{Client, Discovery, Scope},
};
use log::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let v = client.apiserver_version().await?;
    info!("api version: {:?}", v);

    let ns_filter = std::env::var("NAMESPACE").ok();

    let discovery = Discovery::new(&client).await?;

    for group in discovery.groups() {
        let ver = group.preferred_version_or_guess();
        for (api_res, extras) in group.resources_by_version(ver) {
            if !extras.operations.list {
                continue;
            }
            let api: Api<DynamicObject> = if extras.scope == Scope::Namespaced {
                if let Some(ns) = &ns_filter {
                    Api::namespaced_with(client.clone(), ns, &api_res)
                } else {
                    Api::all_with(client.clone(), &api_res)
                }
            } else {
                Api::all_with(client.clone(), &api_res)
            };

            info!("{}/{} : {}", group.name(), ver, api_res.kind);

            let list = api.list(&Default::default()).await?;
            for item in list.items {
                let name = item.name();
                let ns = item.metadata.namespace.map(|s| s + "/").unwrap_or_default();
                info!("\t\t{}{}", ns, name);
            }
        }
    }

    Ok(())
}
