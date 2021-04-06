//! In this example we will implement something similar
//! to `kubectl get all --all-namespaces`.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;
use kube::{
    api::{Api, DynamicObject, GroupVersionKind, ResourceExt},
    Client,
};
use log::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let v = client.apiserver_version().await?;
    info!("api version: {:?}", v);

    let ns_filter = std::env::var("NAMESPACE").ok();

    // The following loops turn the /api or /apis listers into kube::api::Resource
    // objects which can be used to make dynamic api calls.
    // This is slightly awkward because of corev1 types
    // and data split over the list types and the inner get calls.

    // loop over all api groups (except core v1)
    let apigroups = client.list_api_groups().await?;
    for g in apigroups.groups {
        warn!("api group: {}", g.name);
        let ver = g
            .preferred_version
            .as_ref()
            .or_else(|| g.versions.first())
            .expect("preferred or versions exists");
        let apis = client.list_api_group_resources(&ver.group_version).await?;
        print_group(&client, &ver.group_version, apis, ns_filter.as_deref()).await?;
    }

    warn!("core/v1 legacy group");
    let coreapis = client.list_core_api_versions().await?;
    assert_eq!(coreapis.versions.len(), 1);
    let corev1 = client.list_core_api_resources(&coreapis.versions[0]).await?;
    print_group(&client, &coreapis.versions[0], corev1, ns_filter.as_deref()).await
}

async fn print_group(
    client: &Client,
    group_version: &str,
    apis: APIResourceList,
    ns_filter: Option<&str>,
) -> anyhow::Result<()> {
    for ar in apis.resources {
        if !ar.verbs.contains(&"list".to_string()) {
            continue;
        }
        let gvk = GroupVersionKind::from_api_resource(&ar, &apis.group_version);
        let api: Api<DynamicObject> = if ar.namespaced {
            if let Some(ns) = ns_filter {
                Api::namespaced_with(client.clone(), ns, &gvk)
            } else {
                Api::all_with(client.clone(), &gvk)
            }
        } else {
            Api::all_with(client.clone(), &gvk)
        };

        let list = api.list(&Default::default()).await?;
        info!("{} : {}", group_version, ar.kind);
        for item in list.items {
            let name = item.name();
            let ns = item.metadata.namespace.map(|s| s + "/").unwrap_or_default();
            info!("\t\t{}{}", ns, name);
        }
    }
    Ok(())
}
