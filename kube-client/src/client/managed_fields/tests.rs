use super::*;
use crate::{Api, Client, client::Body};
use futures::{FutureExt, TryStreamExt};
use http::Response;
use http_body_util::BodyExt;
use k8s_openapi::api::core::v1::{ConfigMap, Namespace, Pod};
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
        let items = value["items"].as_array().expect("expected a list response");
        assert!(
            items.iter().all(|item| !has_fields(item)),
            "server returned managedFields: {value}"
        );
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
    let result = std::panic::AssertUnwindSafe(async {
        let full: Api<ConfigMap> = Api::namespaced(client.clone(), &ns);
        let stripped = full.clone().without_managed_fields();
        let pp = PostParams {
            field_manager: Some("kube-rs-test".into()),
            ..Default::default()
        };
        let cm: ConfigMap = serde_json::from_value(json!({
            "metadata": {"name": "probe"}, "data": {"key": "value"}
        }))?;
        full.create(&pp, &cm).await?;
        let control = full.get("probe").await?;
        assert!(!control.metadata.managed_fields.as_ref().unwrap().is_empty());
        assert!(stripped.get("probe").await?.metadata.managed_fields.is_none());
        let lp = ListParams::default().fields("metadata.name=probe");
        let listed = stripped.list(&lp).await?;
        assert_eq!(listed.items.len(), 1);
        assert!(listed.items[0].metadata.managed_fields.is_none());
        let wp = WatchParams::default().timeout(1).fields("metadata.name=probe");
        // Strict mode collects the response inside watch(), so bound setup as well.
        let event = tokio::time::timeout(std::time::Duration::from_secs(15), async {
            let events = stripped.watch(&wp, "0").await?;
            std::pin::pin!(events).try_next().await
        })
        .await??;
        match event {
            Some(WatchEvent::Added(object)) => assert!(object.metadata.managed_fields.is_none()),
            event => panic!("expected initial watch object, got {event:?}"),
        }
        Ok::<_, tower::BoxError>(())
    })
    .catch_unwind()
    .await;
    let cleanup = namespaces.delete(&ns, &DeleteParams::default()).await;
    match result {
        Err(panic) => std::panic::resume_unwind(panic),
        Ok(result) => {
            result?;
            cleanup?;
            Ok(())
        }
    }
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
}

#[test]
fn watch_filtering_does_not_depend_on_envelope_field_order() {
    let policy = Some(DropManagedFields { list: false });
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
}

#[test]
fn filtering_rejects_malformed_json_and_trailing_input() {
    let policy = Some(DropManagedFields { list: false });
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
}

#[test]
fn decoding_without_opt_in_preserves_managed_fields() {
    let object = json!({"metadata":{"managedFields":[{}]}});
    assert_eq!(decode::<Value>(&object.to_string(), None, false).unwrap(), object);
}

#[test]
fn list_filtering_preserves_pagination_metadata() {
    let object = json!({"metadata":{"managedFields":[{}], "name":"probe"}});
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
}

#[test]
fn watch_filtering_preserves_error_events_and_nested_data() {
    let object = json!({"metadata":{"managedFields":[{}]},
        "spec":{"metadata":{"managedFields":["user data"]}}});
    for event in ["ADDED", "MODIFIED", "DELETED", "BOOKMARK", "ERROR"] {
        let value = json!({"type":event,"object":object});
        let result: Value =
            decode(&value.to_string(), Some(DropManagedFields { list: false }), true).unwrap();
        assert_eq!(has_fields(&result["object"]), event == "ERROR");
        assert_eq!(result["object"]["spec"], object["spec"]);
    }
}

fn pod_with_managed_fields() -> Value {
    json!({
        "apiVersion":"v1", "kind":"Pod",
        "metadata":{"name":"probe", "managedFields":[{
            "manager":"test", "operation":"Update", "apiVersion":"v1"
        }]},
        "spec":{"containers":[]}
    })
}

const DROP_ACCEPT: &str = "application/json;drop=metadata.managedFields, application/json";

// Each scenario supplies its response explicitly
fn responding_client(
    response: Value,
    check: impl Fn(Request<Body>) -> futures::future::BoxFuture<'static, ()> + Send + Sync + 'static,
) -> Client {
    let service = tower::service_fn(move |request: Request<Body>| {
        let body = format!("{response}\n");
        let check = check(request);
        async move {
            check.await;
            Ok::<_, tower::BoxError>(Response::new(Body::from(body.into_bytes())))
        }
    });
    Client::new(service, "default")
}

fn stripped_api(response: Value) -> Api<Pod> {
    let client = responding_client(response, |request| {
        async move {
            assert_eq!(request.headers()[ACCEPT], DROP_ACCEPT);
        }
        .boxed()
    });
    Api::namespaced(client, "default").without_managed_fields()
}

#[tokio::test]
async fn opt_in_is_local_to_the_api_clone() {
    let client = responding_client(pod_with_managed_fields(), |request| {
        async move {
            if request.extensions().get::<DropManagedFields>().is_some() {
                assert_eq!(request.headers()[ACCEPT], DROP_ACCEPT);
            } else {
                assert!(!request.headers().contains_key(ACCEPT));
            }
        }
        .boxed()
    });
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
    assert!(full.get("probe").await.unwrap().metadata.managed_fields.is_some());
}

#[tokio::test]
async fn list_falls_back_when_the_server_returns_managed_fields() {
    let stripped = stripped_api(json!({"items":[pod_with_managed_fields()]}));
    assert!(
        stripped.list(&ListParams::default()).await.unwrap().items[0]
            .metadata
            .managed_fields
            .is_none()
    );
}

#[tokio::test]
async fn replace_preserves_the_request_body_and_filters_the_response() {
    let pod: Pod = serde_json::from_value(pod_with_managed_fields()).unwrap();
    let expected = serde_json::to_value(&pod).unwrap();
    let client = responding_client(pod_with_managed_fields(), move |request| {
        let expected = expected.clone();
        async move {
            assert_eq!(request.method(), http::Method::PUT);
            assert_eq!(request.headers()[ACCEPT], DROP_ACCEPT);
            let bytes = request.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
        }
        .boxed()
    });
    let stripped: Api<Pod> = Api::namespaced(client, "default").without_managed_fields();
    assert!(
        stripped
            .replace("probe", &PostParams::default(), &pod)
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_none()
    );
}

#[tokio::test]
async fn status_subresource_response_is_filtered() {
    assert!(
        stripped_api(pod_with_managed_fields())
            .get_status("probe")
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_none()
    );
}

#[tokio::test]
async fn create_response_is_filtered() {
    let pod = serde_json::from_value(pod_with_managed_fields()).unwrap();
    let created = stripped_api(pod_with_managed_fields())
        .create(&PostParams::default(), &pod)
        .await
        .unwrap();
    assert!(created.metadata.managed_fields.is_none());
}

#[tokio::test]
async fn patch_response_is_filtered() {
    let patched = stripped_api(pod_with_managed_fields())
        .patch(
            "probe",
            &PatchParams::default(),
            &Patch::Merge(json!({"metadata":{"labels":{"test":"true"}}})),
        )
        .await
        .unwrap();
    assert!(patched.metadata.managed_fields.is_none());
}

#[tokio::test]
async fn patch_status_response_is_filtered() {
    let patched = stripped_api(pod_with_managed_fields())
        .patch_status(
            "probe",
            &PatchParams::default(),
            &Patch::Merge(json!({"status":{}})),
        )
        .await
        .unwrap();
    assert!(patched.metadata.managed_fields.is_none());
}

#[rstest::rstest]
#[case::get(false)]
#[case::patch(true)]
#[tokio::test]
async fn scale_subresource_response_is_filtered(#[case] patch: bool) {
    let api = stripped_api(json!({
        "apiVersion":"autoscaling/v1", "kind":"Scale",
        "metadata":{"name":"probe", "managedFields":[{}]},
        "spec":{"replicas":0}, "status":{"replicas":0}
    }));
    let scale = if patch {
        api.patch_scale(
            "probe",
            &PatchParams::default(),
            &Patch::Merge(json!({"spec":{"replicas":0}})),
        )
        .await
        .unwrap()
    } else {
        api.get_scale("probe").await.unwrap()
    };
    assert!(scale.metadata.managed_fields.is_none());
}

#[tokio::test]
async fn delete_filters_a_returned_object() {
    match stripped_api(pod_with_managed_fields())
        .delete("probe", &DeleteParams::default())
        .await
        .unwrap()
    {
        either::Either::Left(pod) => assert!(pod.metadata.managed_fields.is_none()),
        other => panic!("expected deleted object, got {other:?}"),
    }
}

#[tokio::test]
async fn delete_collection_filters_a_returned_list() {
    match stripped_api(json!({"items":[pod_with_managed_fields()]}))
        .delete_collection(&DeleteParams::default(), &ListParams::default())
        .await
        .unwrap()
    {
        either::Either::Left(list) => assert!(list.items[0].metadata.managed_fields.is_none()),
        other => panic!("expected deleted list, got {other:?}"),
    }
}

#[tokio::test]
async fn watch_falls_back_when_the_server_returns_managed_fields() {
    let stripped = stripped_api(json!({"type":"ADDED", "object":pod_with_managed_fields()}));
    let events = stripped.watch(&WatchParams::default(), "0").await.unwrap();
    let mut events = std::pin::pin!(events);
    match events.try_next().await.unwrap().unwrap() {
        WatchEvent::Added(pod) => assert!(pod.metadata.managed_fields.is_none()),
        event => panic!("unexpected event {event:?}"),
    }
}

#[rstest::rstest]
#[case::object(false)]
#[case::list(true)]
#[tokio::test]
async fn metadata_negotiation_preserves_the_requested_representation(#[case] list: bool) {
    let kind = if list {
        "PartialObjectMetadataList"
    } else {
        "PartialObjectMetadata"
    };
    let accept = format!("application/json;as={kind};g=meta.k8s.io;v=v1");
    let item = json!({"apiVersion":"meta.k8s.io/v1", "kind":"PartialObjectMetadata", "metadata":{"name":"probe", "managedFields":[{}]}});
    let response = if list {
        json!({"apiVersion":"meta.k8s.io/v1", "kind":kind, "items":[item]})
    } else {
        item
    };
    let client = responding_client(response, move |request| {
        let expected = format!("{accept};drop=metadata.managedFields, {accept}");
        async move {
            assert_eq!(request.headers()[ACCEPT], expected);
        }
        .boxed()
    });
    let api: Api<PartialObjectMeta<Pod>> = Api::namespaced(client, "default").without_managed_fields();
    if !list {
        assert!(api.get("probe").await.unwrap().metadata.managed_fields.is_none());
    } else {
        assert!(
            api.list(&ListParams::default()).await.unwrap().items[0]
                .metadata
                .managed_fields
                .is_none()
        );
    }
}

#[tokio::test]
async fn delete_accepts_a_status_response() {
    let api = stripped_api(json!({"apiVersion":"v1", "kind":"Status", "status":"Success", "code":200}));
    assert!(
        api.delete("probe", &DeleteParams::default())
            .await
            .unwrap()
            .is_right()
    );
}
