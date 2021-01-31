#[macro_use] extern crate log;
use color_eyre::{Report, Result};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::{ListParams, Meta, Patch, PatchParams},
    Api, Client, CustomResource,
};
use kube_runtime::controller::{Context, Controller, ReconcilerAction};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use std::collections::BTreeMap;
use tokio::time::Duration;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("Failed to create ConfigMap: {}", source))]
    ConfigMapCreationFailed {
        source: kube::Error,
        backtrace: Backtrace,
    },
    MissingObjectKey {
        name: &'static str,
        backtrace: Backtrace,
    },
}

#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(group = "nullable.se", version = "v1", kind = "ConfigMapGenerator")]
#[kube(shortname = "cmg", namespaced)]
struct ConfigMapGeneratorSpec {
    content: String,
}

fn object_to_owner_reference<K: Meta>(meta: ObjectMeta) -> Result<OwnerReference, Error> {
    Ok(OwnerReference {
        api_version: K::API_VERSION.to_string(),
        kind: K::KIND.to_string(),
        name: meta.name.context(MissingObjectKey {
            name: ".metadata.name",
        })?,
        uid: meta.uid.context(MissingObjectKey {
            name: ".metadata.backtrace",
        })?,
        ..OwnerReference::default()
    })
}

/// Controller triggers this whenever our main object or our children changed
async fn reconcile(generator: ConfigMapGenerator, ctx: Context<Data>) -> Result<ReconcilerAction, Error> {
    let client = ctx.get_ref().client.clone();

    let mut contents = BTreeMap::new();
    contents.insert("content".to_string(), generator.spec.content);
    let cm = ConfigMap {
        metadata: ObjectMeta {
            name: generator.metadata.name.clone(),
            owner_references: Some(vec![OwnerReference {
                controller: Some(true),
                ..object_to_owner_reference::<ConfigMapGenerator>(generator.metadata.clone())?
            }]),
            ..ObjectMeta::default()
        },
        data: Some(contents),
        ..Default::default()
    };
    let mut cm_api = Api::<ConfigMap>::namespaced(
        client.clone(),
        generator.metadata.namespace.as_ref().context(MissingObjectKey {
            name: ".metadata.namespace",
        })?,
    );
    cm_api
        .patch(
            cm.metadata.name.as_ref().context(MissingObjectKey {
                name: ".metadata.name",
            })?,
            &PatchParams::apply("configmapgenerator.kube-rt.nullable.se"),
            &Patch::Apply(&cm),
        )
        .await
        .context(ConfigMapCreationFailed)?;
    Ok(ReconcilerAction {
        requeue_after: Some(Duration::from_secs(300)),
    })
}

/// The controller triggers this on reconcile errors
fn error_policy(_error: &Error, _ctx: Context<Data>) -> ReconcilerAction {
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(1)),
    }
}

// Data we want access to in error/reconcile calls
struct Data {
    client: Client,
}

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "info,kube-runtime=debug,kube=debug");
    env_logger::init();
    let client = Client::try_default().await?;

    let cmgs = Api::<ConfigMapGenerator>::all(client.clone());
    let cms = Api::<ConfigMap>::all(client.clone());

    Controller::new(cmgs, ListParams::default())
        .owns(cms, ListParams::default())
        .run(reconcile, error_policy, Context::new(Data { client }))
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {}", Report::from(e)),
            }
        })
        .await;

    Ok(())
}
