use color_eyre::{Report, Result};
use futures::{stream, StreamExt};
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::Meta,
    api::{ListParams, PatchParams, PatchStrategy},
    Api, Client, Config,
};
use kube_derive::CustomResource;
use kube_runtime::{
    controller::{controller, trigger_owners, trigger_self, ReconcilerAction},
    reflector,
    utils::{try_flatten_addeds, try_flatten_toucheds},
    watcher,
};
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

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::infer().await?;
    let client = Client::new(config);

    let store = reflector::store::Writer::<ConfigMapGenerator>::default();
    controller(
        |generator| {
            let client = client.clone();
            async move {
                let mut contents = BTreeMap::new();
                contents.insert("content".to_string(), generator.spec.content);
                let cm = ConfigMap {
                    metadata: Some(ObjectMeta {
                        name: generator.metadata.name.clone(),
                        owner_references: Some(vec![OwnerReference {
                            controller: Some(true),
                            ..object_to_owner_reference::<ConfigMapGenerator>(
                                generator.metadata.clone(),
                            )?
                        }]),
                        ..ObjectMeta::default()
                    }),
                    data: Some(contents),
                    ..Default::default()
                };
                let cm_api = Api::<ConfigMap>::namespaced(
                    client.clone(),
                    generator
                        .metadata
                        .namespace
                        .as_ref()
                        .context(MissingObjectKey {
                            name: ".metadata.namespace",
                        })?,
                );
                cm_api
                    .patch(
                        cm.metadata.as_ref().and_then(|x| x.name.as_ref()).context(
                            MissingObjectKey {
                                name: ".metadata.name",
                            },
                        )?,
                        &PatchParams {
                            patch_strategy: PatchStrategy::Apply,
                            field_manager: Some(
                                "configmapgenerator.kube-rt.nullable.se".to_string(),
                            ),
                            dry_run: false,
                            force: false,
                        },
                        serde_json::to_vec(&cm).context(SerializationFailed)?,
                    )
                    .await
                    .context(ConfigMapCreationFailed)?;
                Ok(ReconcilerAction {
                    requeue_after: Some(Duration::from_secs(120)),
                })
            }
        },
        |_error: &Error| ReconcilerAction {
            requeue_after: Some(Duration::from_secs(1)),
        },
        store.as_reader(),
        stream::select(
            trigger_self(try_flatten_addeds(reflector(
                store,
                watcher(
                    Api::<ConfigMapGenerator>::all(client.clone()),
                    ListParams::default(),
                ),
            ))),
            trigger_owners(try_flatten_toucheds(watcher(
                Api::<ConfigMap>::all(client.clone()),
                ListParams::default(),
            ))),
        ),
    )
    .for_each(|res| async move { println!("I did a thing! {:?}", res.map_err(Report::from)) })
    .await;

    Ok(())
}
