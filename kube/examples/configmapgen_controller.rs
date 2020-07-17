use color_eyre::{Report, Result};
use futures::{stream, Stream, StreamExt};
use k8s_openapi::{
    api::core::v1::ConfigMap,
    apimachinery::pkg::apis::meta::v1::{ObjectMeta, OwnerReference},
};
use kube::{
    api::{ListParams, Meta, PatchParams, PatchStrategy},
    Api, Client, Config,
};
use kube_derive::CustomResource;
use kube_runtime::{
    controller::{controller, trigger_owners, trigger_self, Context, ReconcilerAction},
    reflector::{reflector, store, ObjectRef},
    utils::{try_flatten_applied, try_flatten_touched},
    watcher,
};
use serde::{Deserialize, Serialize};
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};
use std::{collections::BTreeMap, pin::Pin};
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
        requeue_after: Some(Duration::from_secs(120)),
    })
}

fn error_policy(_error: &Error, _ctx: Context<Data>) -> ReconcilerAction {
    ReconcilerAction {
        requeue_after: Some(Duration::from_secs(1)),
    }
}

struct Data {
    client: Client,
}

type ResultType = Result<ObjectRef<ConfigMapGenerator>, kube_runtime::watcher::Error>;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::infer().await?;
    let client = Client::new(config);
    let context = Context::new(Data {
        client: client.clone(),
    });

    let store = store::Writer::<ConfigMapGenerator>::default();
    let reader = store.as_reader();
    let inputs: Vec<Pin<Box<dyn Stream<Item = ResultType>>>> = vec![
        // NB: don't flatten_touched because ownerrefs take care of delete events
        Box::pin(trigger_self(try_flatten_applied(reflector(
            store,
            watcher(
                Api::<ConfigMapGenerator>::all(client.clone()),
                ListParams::default(),
            ),
        )))),
        // Always trigger CMG whenever the child is applied or deleted!
        Box::pin(trigger_owners(try_flatten_touched(watcher(
            Api::<ConfigMap>::all(client.clone()),
            ListParams::default(),
        )))),
    ];
    let input_stream = stream::select_all(inputs);
    controller(
        reconcile,
        error_policy,
        context,
        reader,
        // The input stream - should produce a stream of ConfigMapGenerator events
        input_stream,
    )
    .for_each(|res| async move { println!("I did a thing! {:?}", res.map_err(Report::from)) })
    .await;

    Ok(())
}
