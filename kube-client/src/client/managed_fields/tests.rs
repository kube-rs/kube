use super::*;
use crate::{Api, Client, client::Body};
use futures::{StreamExt, TryStreamExt};
use http::Response;
use http_body_util::BodyExt;
use k8s_openapi::api::{
    apps::v1::Deployment,
    core::v1::{ConfigMap, Namespace, Pod},
};
use kube_core::{
    PartialObjectMeta, WatchEvent,
    params::{DeleteParams, ListParams, Patch, PatchParams, PostParams, WatchParams},
};
use serde_json::{Value, json};

fn has_fields(object: &Value) -> bool {
    object.pointer("/metadata/managedFields").is_some()
}

fn assert_wire_stripped(value: &Value, watch: bool, list: bool) {
    if watch {
        assert!(
            !has_fields(&value["object"]),
            "server returned managedFields: {value}"
        );
    } else if list {
        if let Some(items) = value["items"].as_array() {
            assert!(
                items.iter().all(|item| !has_fields(item)),
                "server returned managedFields: {value}"
            );
        }
    } else {
        assert!(!has_fields(value), "server returned managedFields: {value}");
    }
}

#[tokio::test]
#[ignore = "requires a Kubernetes cluster; KUBE_RS_REQUIRE_MANAGED_FIELDS_DROP=1 requires KEP-5958 enabled"]
async fn managed_fields_cluster() -> Result<(), tower::BoxError> {
    let require_server = std::env::var("KUBE_RS_REQUIRE_MANAGED_FIELDS_DROP").as_deref() == Ok("1");
    let underlying = Client::try_default().await?;
    // Observe the bytes before kube-rs deserialization/fallback. The strict run
    // must fail if the server ignores the header, even if fallback would work.
    let service = tower::service_fn(move |request: Request<Body>| {
        let client = underlying.clone();
        async move {
            let policy = request.extensions().get::<DropManagedFields>().copied();
            let watch = request.uri().query().is_some_and(|q| q.contains("watch=true"));
            if policy.is_some() {
                assert!(
                    request.headers()[ACCEPT]
                        .to_str()?
                        .contains(";drop=metadata.managedFields")
                );
            }
            let response = client.send(request).await?;
            if require_server
                && response.status().is_success()
                && let Some(policy) = policy
            {
                let (parts, body) = response.into_parts();
                let bytes = body.collect().await?.to_bytes();
                if watch {
                    for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
                        assert_wire_stripped(&serde_json::from_slice(line)?, true, false);
                    }
                } else {
                    assert_wire_stripped(&serde_json::from_slice(&bytes)?, false, policy.list);
                }
                Ok::<_, tower::BoxError>(Response::from_parts(parts, Body::from(bytes)))
            } else {
                Ok(response)
            }
        }
    });
    let client = Client::new(service, "default");
    let namespaces: Api<Namespace> = Api::all(client.clone());
    let ns = namespaces
        .create(
            &PostParams::default(),
            &serde_json::from_value(json!({
                "metadata": {"generateName": "kube-managed-fields-"}
            }))?,
        )
        .await?
        .metadata
        .name
        .unwrap();
    let result = async {
        let full: Api<ConfigMap> = Api::namespaced(client.clone(), &ns);
        let stripped = full.clone().without_managed_fields().constrain(&ns);
        let pp = PostParams { field_manager: Some("kube-rs-test".into()), ..Default::default() };
        let cm: ConfigMap = serde_json::from_value(json!({
            "metadata": {"name": "probe"}, "data": {"key": "value"}
        }))?;
        assert!(stripped.create(&pp, &cm).await?.metadata.managed_fields.is_none());
        let control = full.get("probe").await?;
        assert!(!control.metadata.managed_fields.as_ref().unwrap().is_empty());
        assert!(stripped.get("probe").await?.metadata.managed_fields.is_none());
        assert!(stripped.get_opt("missing").await?.is_none());
        assert!(stripped.get_metadata("probe").await?.metadata.managed_fields.is_none());
        let metadata: Api<PartialObjectMeta<ConfigMap>> = Api::namespaced(client.clone(), &ns).without_managed_fields();
        assert!(metadata.get("probe").await?.metadata.managed_fields.is_none());
        let lp = ListParams::default().fields("metadata.name=probe");
        let listed = stripped.list(&lp).await?;
        assert_eq!(listed.items.len(), 1);
        assert!(listed.items[0].metadata.managed_fields.is_none());
        assert!(stripped.list_metadata(&lp).await?.items[0].metadata.managed_fields.is_none());
        let mut replacement = control;
        replacement.data.as_mut().unwrap().insert("key".into(), "replaced".into());
        assert!(stripped.replace("probe", &pp, &replacement).await?.metadata.managed_fields.is_none());
        let patch = PatchParams::apply("kube-rs-test").force();
        let apply = json!({"apiVersion":"v1", "kind":"ConfigMap", "metadata":{"name":"probe"}, "data":{"key":"applied"}});
        assert!(stripped.patch("probe", &patch, &Patch::Apply(&apply)).await?.metadata.managed_fields.is_none());
        let wp = WatchParams::default().timeout(1).fields("metadata.name=probe");
        let events = stripped.watch(&wp, "0").await?;
        let mut events = std::pin::pin!(events);
        match tokio::time::timeout(std::time::Duration::from_secs(15), events.try_next()).await?? {
            Some(WatchEvent::Added(object)) => assert!(object.metadata.managed_fields.is_none()),
            event => panic!("expected initial watch object, got {event:?}"),
        }
        let events = stripped.watch_metadata(&wp, "0").await?;
        let mut events = std::pin::pin!(events);
        match tokio::time::timeout(std::time::Duration::from_secs(15), events.try_next()).await?? {
            Some(WatchEvent::Added(object)) => assert!(object.metadata.managed_fields.is_none()),
            event => panic!("expected initial metadata watch object, got {event:?}"),
        }
        let deployments: Api<Deployment> = Api::namespaced(client.clone(), &ns);
        let deployment: Deployment = serde_json::from_value(json!({
            "metadata":{"name":"probe"},
            "spec":{"replicas":0,"paused":true,"selector":{"matchLabels":{"app":"probe"}},
                "template":{"metadata":{"labels":{"app":"probe"}},"spec":{"containers":[{"name":"probe","image":"busybox:stable"}]}}}
        }))?;
        deployments.create(&pp, &deployment).await?;
        assert!(!deployments.get_status("probe").await?.metadata.managed_fields.unwrap().is_empty());
        let deployments = deployments.without_managed_fields();
        assert!(deployments.get_status("probe").await?.metadata.managed_fields.is_none());
        assert!(deployments.patch_status("probe", &PatchParams::default(), &Patch::Merge(json!({"status":{"observedGeneration":1}}))).await?.metadata.managed_fields.is_none());
        assert!(deployments.get_scale("probe").await?.metadata.managed_fields.is_none());
        assert!(deployments.patch_scale("probe", &PatchParams::default(), &Patch::Merge(json!({"spec":{"replicas":0}}))).await?.metadata.managed_fields.is_none());
        stripped.delete("probe", &DeleteParams::default()).await?;
        stripped.create(&pp, &cm).await?;
        stripped.delete_collection(&DeleteParams::default(), &lp).await?;
        Ok::<_, tower::BoxError>(())
    }.await;
    namespaces.delete(&ns, &DeleteParams::default()).await?;
    result
}

#[test]
fn escaped_annotations_without_managed_fields_deserialize_normally() {
    let object = json!({"metadata":{"name":"probe", "annotations":{
        "kubectl.kubernetes.io/last-applied-configuration":"{\"kind\":\"Pod\"}",
        "multiline":"first\nsecond"}}, "spec":{"containers":[]}});
    for (value, list, watch) in [
        (object.clone(), false, false),
        (json!({"items":[object.clone()]}), true, false),
        (json!({"type":"ADDED", "object":object.clone()}), false, true),
    ] {
        let text = value.to_string();
        let policy = DropManagedFields { list };
        assert_eq!(decode::<Value>(&text, Some(policy), watch).unwrap(), value);
    }
    let pod: Pod = decode(
        &object.to_string(),
        Some(DropManagedFields { list: false }),
        false,
    )
    .unwrap();
    assert_eq!(pod.metadata.name.as_deref(), Some("probe"));
}

#[test]
fn adapter_skips_fields_before_the_target_deserializer_and_preserves_borrowing() {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Metadata<'a> {
        name: &'a str,
    }
    #[derive(Deserialize)]
    struct Object<'a> {
        #[serde(borrow)]
        metadata: Option<Metadata<'a>>,
    }
    #[derive(Deserialize)]
    struct Wrapped<'a>(#[serde(borrow)] Object<'a>);

    let text = r#"{"metadata":{"managed\u0046ields":{"large":[1,2,3]},"name":"probe"}}"#;
    // The target rejects unknown fields, and its &str must borrow directly
    // from the input. Exercise both newtype and Option forwarding as well.
    assert!(serde_json::from_str::<Wrapped<'_>>(text).is_err());
    let mut deserializer = serde_json::Deserializer::from_str(text);
    let object = Wrapped::deserialize(Filter {
        inner: &mut deserializer,
        path: Path::Object,
    })
    .unwrap();
    deserializer.end().unwrap();
    assert_eq!(object.0.metadata.unwrap().name, "probe");
}

#[test]
fn filtering_handles_field_positions_escaped_keys_and_duplicates() {
    let policy = Some(DropManagedFields { list: false });
    for metadata in [
        r#"{"managedFields": []}"#,
        r#"{"managedFields": [], "name":"probe"}"#,
        r#"{"name":"probe", "managedFields": []}"#,
        r#"{"name":"probe", "managedFields": [], "namespace":"default"}"#,
        r#"{"managedFields": [], "managed\u0046ields": []}"#,
        r#"{"managedFields": [], "name":"probe", "managedFields": []}"#,
        r#"{ "managed\u0046ields" : {"nested":[1,2,3]}, "name":"probe" }"#,
    ] {
        let text = format!(r#"{{"metad\u0061ta":{metadata}}}"#);
        let mut expected: Value = serde_json::from_str(&text).unwrap();
        expected["metadata"]
            .as_object_mut()
            .unwrap()
            .remove("managedFields");
        assert_eq!(decode::<Value>(&text, policy, false).unwrap(), expected, "{text}");
    }
    // Event discrimination must not depend on the order of envelope fields.
    for text in [
        r#"{"object":{"metadata":{"managedFields":[]}},"type":"ADDED"}"#,
        r#"{"type":"ADDED","object":{"metadata":{"managedFields":[]}}}"#,
    ] {
        assert!(!has_fields(
            &decode::<Value>(text, policy, true).unwrap()["object"]
        ));
        let WatchEvent::Added(pod) = decode::<WatchEvent<Pod>>(text, policy, true).unwrap() else {
            panic!("expected an added Pod");
        };
        assert!(pod.metadata.managed_fields.is_none());
    }
    for text in [
        r#"{"metadata":{"managedFields":[}}"#,
        r#"{"metadata":{"managedFields":[]}} trailing"#,
    ] {
        assert!(decode::<Value>(text, policy, false).is_err());
    }
}

#[test]
fn fallback_only_strips_resource_metadata() {
    let object = json!({"metadata":{"managedFields":[{}], "name":"probe"},
        "spec":{"metadata":{"managedFields":["user data"]}},
        "items":[{"metadata":{"managedFields":["user data"]}}]});
    let text = object.to_string();
    let result: Value = decode(&text, Some(DropManagedFields { list: false }), false).unwrap();
    assert!(!has_fields(&result));
    assert_eq!(result["spec"], object["spec"]);
    assert_eq!(result["items"], object["items"]);
    assert_eq!(decode::<Value>(&text, None, false).unwrap(), object);
    let list = json!({"items":[object.clone(), object.clone()],"metadata":{"resourceVersion":"42","continue":"next"}});
    let result: Value = decode(&list.to_string(), Some(DropManagedFields { list: true }), false).unwrap();
    assert!(
        result["items"]
            .as_array()
            .unwrap()
            .iter()
            .all(|item| !has_fields(item))
    );
    assert_eq!(result["metadata"], list["metadata"]);
    for event in ["ADDED", "MODIFIED", "DELETED", "BOOKMARK", "ERROR"] {
        let value = json!({"type":event,"object":object});
        let result: Value =
            decode(&value.to_string(), Some(DropManagedFields { list: false }), true).unwrap();
        assert_eq!(has_fields(&result["object"]), event == "ERROR");
        assert_eq!(result["object"]["spec"], object["spec"]);
    }
    let escaped = r#"{"metadata":{"managed\u0046ields":[{}]}}"#;
    assert!(!has_fields(
        &decode::<Value>(escaped, Some(DropManagedFields { list: false }), false).unwrap()
    ));
}

#[tokio::test]
async fn fallback_handles_old_servers_and_preserves_request_bodies() {
    let object = json!({"apiVersion":"v1","kind":"Pod","metadata":{"name":"probe","managedFields":[{"manager":"test","operation":"Update","apiVersion":"v1"}]},"spec":{"containers":[]}});
    let response_object = object.clone();
    let service = tower::service_fn(move |request: Request<Body>| {
        let object = response_object.clone();
        async move {
            let opted_in = request.extensions().get::<DropManagedFields>().is_some();
            if opted_in {
                assert_eq!(
                    request.headers()[ACCEPT],
                    "application/json;drop=metadata.managedFields, application/json"
                );
            } else {
                assert!(!request.headers().contains_key(ACCEPT));
            }
            let method = request.method().clone();
            let is_list = request.uri().path().ends_with("/pods");
            let watch = request.uri().query().is_some_and(|q| q.contains("watch=true"));
            if method == http::Method::PUT {
                let bytes = request.into_body().collect().await?.to_bytes();
                assert!(has_fields(&serde_json::from_slice(&bytes)?));
            }
            let body = if watch {
                format!("{}\n", json!({"type":"ADDED", "object":object}))
            } else if is_list {
                json!({"apiVersion":"v1","kind":"PodList","metadata":{"resourceVersion":"1"},"items":[object]}).to_string()
            } else {
                object.to_string()
            };
            Ok::<_, tower::BoxError>(Response::new(Body::from(body.into_bytes())))
        }
    });
    let client = Client::new(service, "default");
    let full: Api<Pod> = Api::namespaced(client, "default");
    let stripped = full.clone().without_managed_fields();
    assert!(full.get("probe").await.unwrap().metadata.managed_fields.is_some());
    assert!(
        stripped
            .clone()
            .get("probe")
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_none()
    );
    assert!(
        stripped.list(&ListParams::default()).await.unwrap().items[0]
            .metadata
            .managed_fields
            .is_none()
    );
    let pod: Pod = serde_json::from_value(object).unwrap();
    assert!(
        stripped
            .replace("probe", &PostParams::default(), &pod)
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_none()
    );
    assert!(
        stripped
            .get_status("probe")
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_none()
    );
    match stripped.delete("probe", &DeleteParams::default()).await.unwrap() {
        either::Either::Left(pod) => assert!(pod.metadata.managed_fields.is_none()),
        other => panic!("expected deleted object, got {other:?}"),
    }
    match stripped
        .delete_collection(&DeleteParams::default(), &ListParams::default())
        .await
        .unwrap()
    {
        either::Either::Left(list) => assert!(list.items[0].metadata.managed_fields.is_none()),
        other => panic!("expected deleted list, got {other:?}"),
    }
    let events = stripped.watch(&WatchParams::default(), "0").await.unwrap();
    let mut events = std::pin::pin!(events);
    match events.next().await.unwrap().unwrap() {
        WatchEvent::Added(pod) => assert!(pod.metadata.managed_fields.is_none()),
        event => panic!("unexpected event {event:?}"),
    }
    assert!(full.get("probe").await.unwrap().metadata.managed_fields.is_some());
}

#[tokio::test]
async fn metadata_negotiation_and_status_responses() {
    let service = tower::service_fn(|request: Request<Body>| async move {
        let operation = *request.extensions().get::<&'static str>().unwrap();
        let body = if operation == "delete" {
            json!({"apiVersion":"v1", "kind":"Status", "status":"Success", "code":200})
        } else {
            let list = operation == LIST;
            let kind = if list {
                "PartialObjectMetadataList"
            } else {
                "PartialObjectMetadata"
            };
            let accept = format!("application/json;as={kind};g=meta.k8s.io;v=v1");
            assert_eq!(
                request.headers()[ACCEPT],
                format!("{accept};drop=metadata.managedFields, {accept}")
            );
            let item = json!({"apiVersion":"meta.k8s.io/v1", "kind":"PartialObjectMetadata", "metadata":{"name":"probe", "managedFields":[{}]}});
            if list {
                json!({"apiVersion":"meta.k8s.io/v1", "kind":kind, "items":[item]})
            } else {
                item
            }
        };
        Ok::<_, tower::BoxError>(Response::new(Body::from(body.to_string().into_bytes())))
    });
    let api: Api<PartialObjectMeta<Pod>> =
        Api::namespaced(Client::new(service, "default"), "default").without_managed_fields();
    assert!(api.get("probe").await.unwrap().metadata.managed_fields.is_none());
    assert!(
        api.list(&ListParams::default()).await.unwrap().items[0]
            .metadata
            .managed_fields
            .is_none()
    );
    assert!(
        api.delete("probe", &DeleteParams::default())
            .await
            .unwrap()
            .is_right()
    );
}
