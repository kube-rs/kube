//! In this example we will implement something similar to `kubectl get all`.

use kube::{
    api::{Api, DynamicObject, ResourceExt},
    discovery::{verbs, Discovery, Scope},
    Client,
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    let discovery = Discovery::new(client.clone()).run().await?;
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                continue;
            }
            let api: Api<DynamicObject> = if caps.scope == Scope::Cluster {
                Api::all_with(client.clone(), &ar)
            } else {
                Api::default_namespaced_with(client.clone(), &ar)
            };

            info!("{}/{} : {}", group.name(), ar.version, ar.kind);

            let list = api.list(&Default::default()).await?;
            for item in list.items {
                let name = item.name_any();
                let ns = item.metadata.namespace.map(|s| s + "/").unwrap_or_default();
                info!("\t\t{}{}", ns, name);
            }
        }
    }

    Ok(())
}
