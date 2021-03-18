//! In this example we will implement something similar
//! to `kubectl get all --all`.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;
use kube::{
    api::{DynamicObject, DynamicResource, Meta},
    Client,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let v = client.apiserver_version().await?;
    println!("api version: {:?}", v);

    // The following loops turn the /api or /apis listers into kube::api::Resource
    // objects which can be used to make dynamic api calls.
    // This is slightly awkward because of corev1 types
    // and data split over the list types and the inner get calls.

    // loop over all api groups (except core v1)
    let apigroups = client.list_api_groups().await?;
    for g in apigroups.groups {
        let ver = g
            .preferred_version
            .as_ref()
            .or_else(|| g.versions.first())
            .expect("preferred or versions exists");
        let apis = client.list_api_group_resources(&ver.group_version).await?;
        dump_group(&client, &ver.group_version, apis).await?;
    }
    // core/v1 has a legacy endpoint
    let coreapis = client.list_core_api_versions().await?;

    assert_eq!(coreapis.versions.len(), 1);
    let corev1 = client.list_core_api_resources(&coreapis.versions[0]).await?;
    dump_group(&client, &coreapis.versions[0], corev1).await?;

    Ok(())
}

async fn dump_group(client: &Client, group_version: &str, apis: APIResourceList) -> anyhow::Result<()> {
    println!("{}", group_version);
    for ar in apis.resources {
        if !ar.verbs.contains(&"list".to_string()) {
            continue;
        }
        if group_version.starts_with("discovery.k8s.io/") && ar.kind == "EndpointSlice" ||
          group_version.starts_with("metrics.k8s.io/")  {
            eprintln!("\tFIXME: skipping kind which would be otherwise incorrectly pluralized");
            continue;
        }
        println!("\t{}", ar.kind);
        let api = DynamicResource::from_api_resource(&ar, &apis.group_version)
            .into_api::<DynamicObject>(client.clone());
        let list = api.list(&Default::default()).await?;
        for item in list.items {
            let name = item.name();
            let ns = item.namespace();
            match ns {
                Some(ns) => {
                    println!("\t\t{}/{}", ns, name);
                }
                None => {
                    println!("\t\t{}", name);
                }
            }
        }
    }

    Ok(())
}
