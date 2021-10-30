#[macro_use] extern crate log;
use color_eyre::{Report, Result};
use futures::StreamExt;
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::{Api, ListParams, Patch, PatchParams, Resource},
    runtime::controller::{Context, Controller, ReconcilerAction},
    Client, CustomResource,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io::BufRead};
use thiserror::Error;
use tokio::time::Duration;

#[derive(Debug, Error)]
enum Error {
    #[error("Failed to create ConfigMap: {0}")]
    ConfigMapCreationFailed(#[source] kube::Error),
    #[error("MissingObjectKey: {name}")]
    MissingObjectKey { name: &'static str },
}

#[derive(CustomResource, Debug, Clone, Deserialize, Serialize, JsonSchema)]
#[kube(group = "nullable.se", version = "v1", kind = "ConfigMapGenerator")]
#[kube(shortname = "cmg", namespaced)]
struct ConfigMapGeneratorSpec {
    content: String,
}

fn object_to_owner_reference<K: Resource<DynamicType = ()>>(
    meta: ObjectMeta,
) -> Result<OwnerReference, Error> {
    Ok(OwnerReference {
        api_version: K::api_version(&()).to_string(),
        kind: K::kind(&()).to_string(),
        name: meta.name.ok_or(Error::MissingObjectKey {
            name: ".metadata.name",
        })?,
        uid: meta.uid.ok_or(Error::MissingObjectKey {
            name: ".metadata.uid",
        })?,
        ..OwnerReference::default()
    })
}

/// Controller triggers this whenever our main object or our children changed
async fn reconcile(generator: ConfigMapGenerator, ctx: Context<Data>) -> Result<ReconcilerAction, Error> {
    log::info!("working hard");
    tokio::time::sleep(Duration::from_secs(2)).await;
    log::info!("hard work is done!");

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
    let cm_api = Api::<ConfigMap>::namespaced(
        client.clone(),
        generator
            .metadata
            .namespace
            .as_ref()
            .ok_or(Error::MissingObjectKey {
                name: ".metadata.namespace",
            })?,
    );
    cm_api
        .patch(
            cm.metadata.name.as_ref().ok_or(Error::MissingObjectKey {
                name: ".metadata.name",
            })?,
            &PatchParams::apply("configmapgenerator.kube-rt.nullable.se"),
            &Patch::Apply(&cm),
        )
        .await
        .map_err(Error::ConfigMapCreationFailed)?;
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

    log::info!("starting configmapgen-controller");
    log::info!("press <enter> to force a reconciliation of all objects");

    let (mut reload_tx, reload_rx) = futures::channel::mpsc::channel(0);
    // Using a regular background thread since tokio::io::stdin() doesn't allow aborting reads,
    // and its worker prevents the Tokio runtime from shutting down.
    std::thread::spawn(move || {
        for _ in std::io::BufReader::new(std::io::stdin()).lines() {
            let _ = reload_tx.try_send(());
        }
    });

    Controller::new(cmgs, ListParams::default())
        .owns(cms, ListParams::default())
        .reconcile_all_on(reload_rx.map(|_| ()))
        .shutdown_on_signal()
        .run(reconcile, error_policy, Context::new(Data { client }))
        .for_each(|res| async move {
            match res {
                Ok(o) => info!("reconciled {:?}", o),
                Err(e) => warn!("reconcile failed: {}", Report::from(e)),
            }
        })
        .await;
    log::info!("controller terminated");
    Ok(())
}
