//! In this example we will implement something similar to `kubectl get all`.

use kube::{
    Client,
    api::{Api, DynamicObject, Namespaces, ResourceExt},
    discovery::{Discovery, verbs},
};
use tracing::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let client = Client::try_default().await?;

    // Uses Aggregated Discovery API for fewer API calls (2 instead of N+2)
    let discovery = Discovery::new(client.clone()).run_aggregated().await?;
    for group in discovery.groups() {
        for (ar, caps) in group.recommended_resources() {
            if !caps.supports_operation(verbs::LIST) {
                continue;
            }
            let api: Api<DynamicObject> = Api::scoped_with(client.clone(), Namespaces::Default, &ar, &caps);

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
