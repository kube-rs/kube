//! In this example we will implement something similar
//! to `kubectl get all --all-namespaces`.

use kube::{
    api::{Api, DynamicObject, ResourceExt},
    discovery::{verbs, Discovery, Scope},
    Client,
};
use log::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;
    let ns_filter = std::env::var("NAMESPACE").ok();

    let discovery = Discovery::new(client.clone()).run().await?;
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                continue;
            }
            let api: Api<DynamicObject> = if caps.scope == Scope::Namespaced {
                if let Some(ns) = &ns_filter {
                    Api::namespaced_with(client.clone(), ns, &ar)
                } else {
                    Api::all_with(client.clone(), &ar)
                }
            } else {
                Api::all_with(client.clone(), &ar)
            };

            info!("{}/{} : {}", group.name(), ar.version, ar.kind);

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
