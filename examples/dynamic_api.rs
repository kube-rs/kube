#[macro_use] extern crate log;
use kube::{api::DynamicResource, Client};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    std::env::set_var("RUST_LOG", "info,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let v = client.apiserver_version().await?;
    info!("api version: {:?}", v);

    // The following loops turn the /api or /apis listers into kube::api::Resource
    // objects which can be used to make dynamic api calls.
    // This is slightly awkward because of corev1 types
    // and data split over the list types and the inner get calls.

    // loop over all api groups (except core v1)
    let apigroups = client.list_api_groups().await?;
    for g in apigroups.groups {
        info!("group: {}", g.name);
        debug!("group: {:?}", g);
        let ver = g
            .preferred_version
            .as_ref()
            .or_else(|| g.versions.first())
            .expect("preferred or versions exists");
        info!("polling: {} at {:?}", g.name, ver);
        let apis = client.list_api_group_resources(&ver.group_version).await?;
        dbg!(&apis);
        for ar in apis.resources {
            let r = DynamicResource::from_api_resource(&ar, &apis.group_version).into_resource();
            dbg!(r);
        }
    }
    // core/v1 has a legacy endpoint
    let coreapis = client.list_core_api_versions().await?;
    for corever in coreapis.versions {
        dbg!(&corever);
        let apis = client.list_core_api_resources(&corever).await?;
        debug!("Got {:?}", apis);
        for cr in apis.resources {
            dbg!(&cr);
            let r = DynamicResource::from_api_resource(&cr, &apis.group_version).into_resource();
            dbg!(r);
        }
    }

    Ok(())
}
