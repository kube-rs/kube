#[macro_use] extern crate log;
use color_eyre::{Report, Result};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::{ListParams, Meta, PatchParams, PatchStrategy},
    Api, Client, Config,
};
use kube_derive::CustomResource;
use kube_runtime::controller::{Context, Controller, ReconcilerAction};
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
    SerializationFailed {
        source: serde_json::Error,
        backtrace: Backtrace,
    },
}

#[derive(CustomResource, Debug, Clone, Deserialize, Serialize)]
#[kube(group = "nullable.se", version = "v1", namespaced)]
#[kube(shortname = "cmg")]
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
        metadata: Some(ObjectMeta {
            name: generator.metadata.name.clone(),
            owner_references: Some(vec![OwnerReference {
                controller: Some(true),
                ..object_to_owner_reference::<ConfigMapGenerator>(generator.metadata.clone())?
            }]),
            ..ObjectMeta::default()
        }),
        data: Some(contents),
        ..Default::default()
    };
    let cm_api = Api::<ConfigMap>::namespaced(
        client.clone(),
        generator.metadata.namespace.as_ref().context(MissingObjectKey {
            name: ".metadata.namespace",
        })?,
    );
    cm_api
        .patch(
            cm.metadata
                .as_ref()
                .and_then(|x| x.name.as_ref())
                .context(MissingObjectKey {
                    name: ".metadata.name",
                })?,
            &PatchParams {
                patch_strategy: PatchStrategy::Apply,
                field_manager: Some("configmapgenerator.kube-rt.nullable.se".to_string()),
                dry_run: false,
                force: false,
            },
            serde_json::to_vec(&cm).context(SerializationFailed)?,
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
    std::env::set_var("RUST_LOG", "info,kube=debug");
    let config = Config::infer().await?;
    let client = Client::new(config);
    let context = Context::new(Data {
        client: client.clone(),
    });

    let cmgs = Api::<ConfigMapGenerator>::all(client.clone());
    let cms = Api::<ConfigMap>::all(client.clone());

    Controller::new(cmgs, ListParams::default())
        .owns(cms, ListParams::default())
        .run(reconcile, error_policy, context)
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {}", Report::from(e)),
            }
        })
        .await;

    Ok(())
}
