use super::*;
use crate::{Api, Client, client::Body};
use futures::TryStreamExt;
use http::Response;
use http_body_util::BodyExt;
use k8s_openapi::api::core::v1::Pod;
use kube_core::{
    PartialObjectMeta, WatchEvent,
    params::{DeleteParams, ListParams, PostParams, WatchParams},
};
use serde_json::{Value, json};

const DROP_ACCEPT: &str = "application/json;drop=metadata.managedFields, application/json";

#[test]
fn drop_gets_added_to_each_accept_and_keeps_original_as_fallback() {
    let accept = "application/json;as=PartialObjectMetadata;g=meta.k8s.io;v=v1, application/json";
    let mut request = Request::get("/").header(ACCEPT, accept).body(vec![]).unwrap();
    prepare(&mut request, true);
    assert_eq!(
        request.headers()[ACCEPT],
        format!(
            "application/json;as=PartialObjectMetadata;g=meta.k8s.io;v=v1;drop=metadata.managedFields, application/json;drop=metadata.managedFields, {accept}"
        )
    );
}

#[tokio::test]
async fn converting_to_client_clears_opt_out_for_discovery_and_other_apis() {
    use kube_core::discovery::v2::ACCEPT_AGGREGATED_DISCOVERY_V2;

    let client = responding_client(json!({"items": []}), Some(ACCEPT_AGGREGATED_DISCOVERY_V2.into()));
    let api = Api::<Pod>::all(client).without_managed_fields();
    Client::from(api).list_api_groups_aggregated().await.unwrap();

    let client = responding_client(pod_with_managed_fields(), None);
    let api = Api::<Pod>::all(client).without_managed_fields();
    let other = Api::<Pod>::namespaced(Client::from(api), "other");
    other.get("probe").await.unwrap();
}

fn pod_with_managed_fields() -> Value {
    json!({"apiVersion":"v1", "kind":"Pod", "metadata":{"name":"probe",
        "managedFields":[{"manager":"test", "operation":"Update", "apiVersion":"v1"}]},
        "spec":{"containers":[]}})
}

fn responding_client(response: Value, accept: Option<String>) -> Client {
    let service = tower::service_fn(move |request: Request<Body>| {
        assert_eq!(
            request.headers().get(ACCEPT).map(|v| v.to_str().unwrap()),
            accept.as_deref()
        );
        let body = format!("{response}\n");
        async move { Ok::<_, tower::BoxError>(Response::new(Body::from(body.into_bytes()))) }
    });
    Client::new(service, "default")
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
#[tokio::test]
async fn opt_in_controls_headers_and_preserves_server_responses(#[case] enabled: bool) {
    let api = |response| {
        let accept = enabled.then(|| DROP_ACCEPT.into());
        let api = Api::<Pod>::namespaced(responding_client(response, accept), "default");
        if enabled {
            api.without_managed_fields()
        } else {
            api
        }
    };
    assert!(
        api(pod_with_managed_fields())
            .get("probe")
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_some()
    );
    assert!(
        api(json!({"items":[pod_with_managed_fields()]}))
            .list(&ListParams::default())
            .await
            .unwrap()
            .items[0]
            .metadata
            .managed_fields
            .is_some()
    );
    let events = api(json!({"type":"ADDED", "object":pod_with_managed_fields()}))
        .watch(&WatchParams::default(), "0")
        .await
        .unwrap();
    let Some(WatchEvent::Added(pod)) = std::pin::pin!(events).try_next().await.unwrap() else {
        panic!("expected an added Pod");
    };
    assert!(pod.metadata.managed_fields.is_some());
    let deleted = api(pod_with_managed_fields())
        .delete("probe", &DeleteParams::default())
        .await
        .unwrap();
    assert!(deleted.left().unwrap().metadata.managed_fields.is_some());
}

#[tokio::test]
async fn opt_in_is_local_to_the_api_clone() {
    let service = tower::service_fn(|request: Request<Body>| async move {
        let opted_in = request
            .headers()
            .get(ACCEPT)
            .is_some_and(|value| value == DROP_ACCEPT);
        let mut response = pod_with_managed_fields();
        if opted_in {
            response["metadata"]
                .as_object_mut()
                .unwrap()
                .remove("managedFields");
        }
        Ok::<_, tower::BoxError>(Response::new(Body::from(response.to_string().into_bytes())))
    });
    let client = Client::new(service, "default");
    let full: Api<Pod> = Api::namespaced(client.clone(), "default");
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
    let other: Api<Pod> = Api::namespaced(client, "other");
    assert!(
        other
            .get("probe")
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_some()
    );
}

#[tokio::test]
async fn policy_does_not_affect_raw_requests() {
    let object = pod_with_managed_fields();
    let api: Api<Pod> =
        Api::namespaced(responding_client(object.clone(), None), "default").without_managed_fields();
    let client: Client = api.into();
    let text = client
        .request_text(
            Request::get("/api/v1/namespaces/default/pods/probe")
                .body(vec![])
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), object);
}

#[tokio::test]
async fn replace_preserves_the_request_body() {
    let pod: Pod = serde_json::from_value(pod_with_managed_fields()).unwrap();
    let expected = serde_json::to_value(&pod).unwrap();
    let service = tower::service_fn(move |request: Request<Body>| {
        let expected = expected.clone();
        async move {
            assert_eq!(request.headers()[ACCEPT], DROP_ACCEPT);
            let bytes = request.into_body().collect().await.unwrap().to_bytes();
            assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
            Ok::<_, tower::BoxError>(Response::new(Body::from(expected.to_string().into_bytes())))
        }
    });
    let api: Api<Pod> = Api::namespaced(Client::new(service, "default"), "default").without_managed_fields();
    assert!(
        api.replace("probe", &PostParams::default(), &pod)
            .await
            .unwrap()
            .metadata
            .managed_fields
            .is_some()
    );
}

#[rstest::rstest]
#[case(false)]
#[case(true)]
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
    let client = responding_client(
        response,
        Some(format!("{accept};drop=metadata.managedFields, {accept}")),
    );
    let api: Api<PartialObjectMeta<Pod>> = Api::namespaced(client, "default").without_managed_fields();
    if list {
        assert!(
            api.list(&ListParams::default()).await.unwrap().items[0]
                .metadata
                .managed_fields
                .is_some()
        );
    } else {
        assert!(api.get("probe").await.unwrap().metadata.managed_fields.is_some());
    }
}
