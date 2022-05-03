use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::{
    api::{Api, Patch, PatchParams},
    core::crd::merge_crds,
    runtime::wait::{await_condition, conditions},
    Client, CustomResource, CustomResourceExt, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::*;

mod v1 {
    use super::*;
    // spec that is forwards compatible with v2 (can upgrade by truncating)
    #[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
    #[kube(group = "kube.rs", version = "v1", kind = "ManyDerive", namespaced)]
    pub struct ManyDeriveSpec {
        pub name: String,
        pub oldprop: u32,
    }
}
mod v2 {
    // spec that is NOT backwards compatible with v1 (cannot retrieve oldprop if truncated)
    use super::*;
    #[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
    #[kube(group = "kube.rs", version = "v2", kind = "ManyDerive", namespaced)]
    pub struct ManyDeriveSpec {
        pub name: String,
        pub extra: Option<String>,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let client = Client::try_default().await?;
    let ssapply = PatchParams::apply("crd_derive_multi").force();

    let crd1 = v1::ManyDerive::crd();
    let crd2 = v2::ManyDerive::crd();
    let all_crds = vec![crd1.clone(), crd2.clone()];

    // apply schema where v1 is the stored version
    apply_crd(client.clone(), merge_crds(all_crds.clone(), "v1")?).await?;

    // create apis
    let v1api: Api<v1::ManyDerive> = Api::default_namespaced(client.clone());
    let v2api: Api<v2::ManyDerive> = Api::default_namespaced(client.clone());

    // create a v1 version
    let v1m = v1::ManyDerive::new("old", v1::ManyDeriveSpec {
        name: "i am old".into(),
        oldprop: 5,
    });
    let oldvarv1 = v1api.patch("old", &ssapply, &Patch::Apply(&v1m)).await?;
    info!("old instance on v1: {:?}", oldvarv1.spec);
    let oldvarv2 = v2api.get("old").await?;
    info!("old instance on v2 truncates: {:?}", oldvarv2.spec);

    // create a v2 version
    let v2m = v2::ManyDerive::new("new", v2::ManyDeriveSpec {
        name: "i am new".into(),
        extra: Some("hi".into()),
    });
    let newvarv2 = v2api.patch("new", &ssapply, &Patch::Apply(&v2m)).await?;
    info!("new instance on v2 is force downgraded: {:?}", newvarv2.spec); // no extra field
    let cannot_fetch_as_old = v1api.get("new").await.unwrap_err();
    info!("cannot fetch new on v1: {:?}", cannot_fetch_as_old);

    // apply schema upgrade
    apply_crd(client.clone(), merge_crds(all_crds, "v2")?).await?;

    // nothing changed with existing objects without conversion
    //let oldvarv1_upg = v1api.get("old").await?;
    //info!("old instance unchanged on v1: {:?}", oldvarv1_upg.spec);
    //let oldvarv2_upg = v2api.get("old").await?;
    //info!("old instance unchanged on v2: {:?}", oldvarv2_upg.spec);

    // re-apply new now that v2 is stored gives us the extra properties
    let newvarv2_2 = v2api.patch("new", &ssapply, &Patch::Apply(&v2m)).await?;
    info!("new on v2 correct on reapply to v2: {:?}", newvarv2_2.spec);


    // note we can apply old versions without them being truncated to the v2 schema
    // in our case this means we cannot fetch them with our v1 schema (breaking change to not have oldprop)
    let v1m2 = v1::ManyDerive::new("old", v1::ManyDeriveSpec {
        name: "i am old2".into(),
        oldprop: 5,
    });
    let v1err = v1api
        .patch("old", &ssapply, &Patch::Apply(&v1m2))
        .await
        .unwrap_err();
    info!("cannot get old on v1 anymore: {:?}", v1err); // mandatory field oldprop truncated
                                                        // ...but the change is still there:
    let old_still_there = v2api.get("old").await?;
    assert_eq!(old_still_there.spec.name, "i am old2");

    cleanup(client.clone()).await?;
    Ok(())
}


async fn apply_crd(client: Client, crd: CustomResourceDefinition) -> anyhow::Result<()> {
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    info!("Creating crd: {}", serde_yaml::to_string(&crd)?);
    let ssapply = PatchParams::apply("crd_derive_multi").force();
    crds.patch("manyderives.kube.rs", &ssapply, &Patch::Apply(&crd))
        .await?;
    let establish = await_condition(
        crds.clone(),
        "manyderives.kube.rs",
        conditions::is_crd_established(),
    );
    let _ = tokio::time::timeout(std::time::Duration::from_secs(10), establish).await?;
    Ok(())
}

async fn cleanup(client: Client) -> anyhow::Result<()> {
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    let obj = crds.delete("manyderives.kube.rs", &Default::default()).await?;
    if let either::Either::Left(o) = obj {
        let uid = o.uid().unwrap();
        await_condition(crds, "manyderives.kube.rs", conditions::is_deleted(&uid)).await?;
    }
    Ok(())
}
