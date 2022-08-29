use std::convert::Infallible;

use k8s_openapi::{
    apiextensions_apiserver::pkg::apis::apiextensions::v1::{
        CustomResourceConversion, CustomResourceDefinition, ServiceReference, WebhookClientConfig,
        WebhookConversion,
    },
    ByteString,
};
use kube::{
    api::{Api, Patch, PatchParams},
    core::{
        conversion::{ConversionHandler, StarConverter},
        crd::merge_crds,
    },
    runtime::wait::{await_condition, conditions},
    Client, CustomResource, CustomResourceExt, ResourceExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tracing::*;
use warp::Filter;

mod v1 {
    use super::*;
    #[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
    #[kube(group = "kube.rs", version = "v1", kind = "ManyDerive", namespaced)]
    pub struct ManyDeriveSpec {
        pub name: String,
        pub oldprop: u32,
    }
}
mod v2 {
    use super::*;
    #[derive(CustomResource, Serialize, Deserialize, Default, Debug, Clone, JsonSchema)]
    #[kube(group = "kube.rs", version = "v2", kind = "ManyDerive", namespaced)]
    pub struct ManyDeriveSpec {
        pub name: String,
        pub newprop: u32,
    }
}

// As you can see, two CRD versions are not compatible on schema level, and default
// custom resource conversion strategy (which simply changes apiVersion)
// will lead to data loss. Therefore we will implement conversion webhook which allows us
// to seamlessly work with the objects using both api versions, allowing for safe and gradual migration.
mod conversion {
    use kube::core::{NotUsed, Object};

    // At first, we need to define internal, unversioned representation of our resource.
    // it never leaks to the outer world, so it can evolve independently of published versions
    // (as long as it is logically compatible with all versions).
    #[derive(Clone)]
    pub struct ManyDeriveSpec {
        pub beautiful_name: String,
        pub prop: u32,
    }
    type ManyDerive = Object<ManyDeriveSpec, NotUsed>;

    pub struct RayV1;
    impl kube::core::conversion::StarRay for RayV1 {
        type Unversioned = ManyDerive;
        type Versioned = super::v1::ManyDerive;

        fn into_unversioned(&self, versioned: Self::Versioned) -> Result<Self::Unversioned, String> {
            let spec = ManyDeriveSpec {
                beautiful_name: versioned.spec.name,
                prop: versioned.spec.oldprop,
            };
            Ok(Object {
                metadata: versioned.metadata,
                spec,
                status: None,
                types: None,
            })
        }

        fn from_unversioned(&self, unversioned: Self::Unversioned) -> Result<Self::Versioned, String> {
            let spec = super::v1::ManyDeriveSpec {
                name: unversioned.spec.beautiful_name,
                oldprop: unversioned.spec.prop,
            };
            Ok(super::v1::ManyDerive {
                metadata: unversioned.metadata,
                spec,
            })
        }
    }

    pub struct RayV2;
    impl kube::core::conversion::StarRay for RayV2 {
        type Unversioned = ManyDerive;
        type Versioned = super::v2::ManyDerive;

        fn into_unversioned(&self, versioned: Self::Versioned) -> Result<Self::Unversioned, String> {
            let spec = ManyDeriveSpec {
                beautiful_name: versioned.spec.name,
                prop: versioned.spec.newprop,
            };
            Ok(Object {
                metadata: versioned.metadata,
                spec,
                status: None,
                types: None,
            })
        }

        fn from_unversioned(&self, unversioned: Self::Unversioned) -> Result<Self::Versioned, String> {
            let spec = super::v2::ManyDeriveSpec {
                name: unversioned.spec.beautiful_name,
                newprop: unversioned.spec.prop,
            };
            Ok(super::v2::ManyDerive {
                metadata: unversioned.metadata,
                spec,
            })
        }
    }
}

// this function actually implements conversion service
async fn run_conversion_webhook() {
    let star_converter = StarConverter::builder()
        .add_ray(conversion::RayV1)
        .add_ray(conversion::RayV2)
        .build();
    let handler = ConversionHandler::new(star_converter);

    let routes = warp::path("convert")
        .and(warp::body::json())
        .and_then(move |req| {
            let result = handler.handle(req);

            async move { Ok::<_, Infallible>(warp::reply::json(&result)) }
        })
        .with(warp::trace::request());

    // You must generate a certificate for the service / url.
    // See admission_setup.sh as a starting point for how to do this, configuration of the
    // conversion and admission webhooks is similar.
    let addr = format!("{}:8443", std::env::var("WEBHOOK_BIND_IP").unwrap());
    warp::serve(warp::post().and(routes))
        .tls()
        .cert_path(std::env::var("WEBHOOK_TLS_CRT").unwrap())
        .key_path(std::env::var("WEBHOOK_TLS_KEY").unwrap())
        .run(addr.parse::<std::net::SocketAddr>().unwrap())
        .await;
}

async fn add_conversion_config(crd: &mut CustomResourceDefinition) -> anyhow::Result<()> {
    // path to the CA root certificate which issued webhook serving certificate
    let ca_crt = tokio::fs::read(std::env::var("CA_PATH").unwrap()).await?;
    let mut client_config = WebhookClientConfig {
        ca_bundle: Some(ByteString(ca_crt)),
        service: None,
        url: None,
    };
    // If you launched webhook outside the cluster (e.g. locally), set WEBHOOK_URL
    // to its url (must end with '/convert', must be reachable from the apiserver).
    if let Ok(url) = std::env::var("WEBHOOK_URL") {
        client_config.url = Some(url);
    } else {
        // If you launched webhook in cluster (e.g. as a Deployment), create a
        // service pointing to webhook pods, and set WEBHOOK_NAMESPACE and
        // WEBHOOK_SVC to the namespace and name of the service.
        // YOu may refer to crd_derive_multi.yaml for example manifests.
        client_config.service = Some(ServiceReference {
            namespace: std::env::var("WEBHOOK_NAMESPACE").unwrap(),
            name: std::env::var("WEBHOOK_SVC").unwrap(),
            port: Some(443),
            path: Some("/convert".to_string()),
        });
    }
    crd.spec.conversion = Some(CustomResourceConversion {
        strategy: "Webhook".to_string(),
        webhook: Some(WebhookConversion {
            client_config: Some(client_config),
            conversion_review_versions: vec!["v1".to_string()],
        }),
    });

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    // to run this example, you need to launch two instances in parallel:
    // one with RUN_WEBHOOK=1 - it will be conversion webhook (also set WEBHOOK_BIND_IP, WEBHOOK_TLS_CRT, WEBHOOK_TLS_KEY)
    // one without - it will be example itself (also set CA_PATH, WEBHOOK_NAMESPACE and WEBHOOK_SVC or WEBHOOK_URL)
    if std::env::var("RUN_WEBHOOK").is_ok() {
        run_conversion_webhook().await;
        return Ok(());
    }

    let client = Client::try_default().await?;
    let ssapply = PatchParams::apply("crd_derive_multi").force();

    let mut crd1 = v1::ManyDerive::crd();
    let mut crd2 = v2::ManyDerive::crd();
    add_conversion_config(&mut crd1).await?;
    add_conversion_config(&mut crd2).await?;
    let mut all_crds = vec![crd1.clone(), crd2.clone()];

    // apply schema where v1 is the stored version
    apply_crd(client.clone(), merge_crds(all_crds.clone(), "v1")?).await?;

    // create apis
    let v1api: Api<v1::ManyDerive> = Api::default_namespaced(client.clone());
    let v2api: Api<v2::ManyDerive> = Api::default_namespaced(client.clone());

    // create a v1 resource
    let v1m = v1::ManyDerive::new("old", v1::ManyDeriveSpec {
        name: "i am old".into(),
        oldprop: 5,
    });
    let oldvarv1 = v1api.patch("old", &ssapply, &Patch::Apply(&v1m)).await?;
    info!("old instance on v1: {:?}", oldvarv1.spec);
    let oldvarv2 = v2api.get("old").await?;
    info!("old instance on v2 not truncated: {:?}", oldvarv2.spec);
    assert_eq!(oldvarv2.spec.newprop, 5); // no data loss

    // create a v2 version
    let v2m = v2::ManyDerive::new("new", v2::ManyDeriveSpec {
        name: "i am new".into(),
        newprop: 4,
    });
    let newvarv2 = v2api.patch("new", &ssapply, &Patch::Apply(&v2m)).await?;
    info!(
        "new instance on v2 was force downgraded on storage: {:?}",
        newvarv2.spec
    );
    assert_eq!(newvarv2.spec.newprop, 4); // no data loss
    let can_fetch_as_old = v1api.get("new").await.unwrap();
    info!(
        "fetched new instance using old api version: {:?}",
        can_fetch_as_old
    );

    // as you can see, proper conversion allows us to access the same object under different
    // apiVersions without any problems. For example, you may have custom controller which uses v1 api
    // and admission webhook which uses v2.

    // Now imagine that we upgraded all our clients to the v2 API
    // and we want to drop support for the legacy v1 API (after all you see how many code we had to write
    // for conversion).

    // at first, we need to promote `v2` version as stored (so that all new or updated resources are persisted
    // in the cluster storage using new version).
    info!("Selecting v2 as storage version");
    apply_crd(client.clone(), merge_crds(all_crds.clone(), "v2")?).await?;
    // here we use `migrate_resources` utility function to migrate all previously stored objects.
    info!("Running storage migration");
    let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
    crds.migrate_resources(&crd2.name_unchecked());
    // and now we can apply CRD again without specifying v1, completely removing it.
    info!("Removing v1");
    all_crds.remove(0);
    apply_crd(client.clone(), merge_crds(all_crds, "v2")?).await?;

    // now it is impossible to use v1
    let v1m2 = v1::ManyDerive::new("old", v1::ManyDeriveSpec {
        name: "i am old2".into(),
        oldprop: 5,
    });
    let v1err = v1api
        .patch("old", &ssapply, &Patch::Apply(&v1m2))
        .await
        .unwrap_err();
    info!("cannot get old on v1 anymore: {:?}", v1err);
    // but objects which were initally stored as v1, were migrated to v2
    // and now are available through the v2 API.
    let old_still_there = v2api.get("old").await?;
    assert_eq!(old_still_there.spec.name, "i am old");

    cleanup(client).await?;
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
