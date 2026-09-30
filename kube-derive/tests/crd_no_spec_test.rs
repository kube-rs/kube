#![allow(missing_docs)]

//! `#[kube(no_spec)]` keeps the derived struct as the Rust-side `spec`, but flattens it onto the
//! root of the custom resource on the wire and in the generated schema.

use assert_json_diff::assert_json_eq;
use kube::{CustomResourceExt, KubeSchema, core::object::HasSpec};
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Bucket",
    namespaced,
    no_spec,
    derive = "PartialEq"
)]
pub struct BucketFields {
    name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    region: Option<String>,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Crate",
    namespaced,
    no_spec,
    status = "CrateStatus",
    derive = "PartialEq"
)]
pub struct CrateFields {
    label: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Default, JsonSchema)]
pub struct CrateStatus {
    shipped: bool,
}

#[test]
fn no_spec_serializes_fields_at_the_root() {
    let bucket = Bucket::new("blobs", BucketFields {
        name: "blobs".into(),
        region: Some("eu-west-1".into()),
    });

    assert_json_eq!(
        serde_json::to_value(&bucket).unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "Bucket",
            "metadata": { "name": "blobs" },
            "name": "blobs",
            "region": "eu-west-1",
        })
    );
}

#[test]
fn no_spec_omits_skipped_fields() {
    let bucket = Bucket::new("blobs", BucketFields {
        name: "blobs".into(),
        region: None,
    });
    let value = serde_json::to_value(&bucket).unwrap();
    assert!(value.get("region").is_none(), "got {value}");
}

#[test]
fn no_spec_round_trips() {
    let bucket = Bucket::new("blobs", BucketFields {
        name: "blobs".into(),
        region: Some("eu-west-1".into()),
    });
    let json = serde_json::to_string(&bucket).unwrap();
    let back: Bucket = serde_json::from_str(&json).unwrap();
    assert_eq!(back, bucket);
    // the derived struct is still reachable as the spec
    assert_eq!(back.spec().name, "blobs");
}

#[test]
fn no_spec_status_stays_a_sibling_of_the_flattened_fields() {
    let mut krate = Crate::new("kube", CrateFields { label: "rust".into() });
    assert_json_eq!(
        serde_json::to_value(&krate).unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "Crate",
            "metadata": { "name": "kube" },
            "label": "rust",
        })
    );

    krate.status = Some(CrateStatus { shipped: true });
    assert_json_eq!(
        serde_json::to_value(&krate).unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "Crate",
            "metadata": { "name": "kube" },
            "label": "rust",
            "status": { "shipped": true },
        })
    );

    let back: Crate = serde_json::from_value(serde_json::to_value(&krate).unwrap()).unwrap();
    assert_eq!(back, krate);
}

#[test]
fn no_spec_hoists_properties_into_the_crd_schema() {
    let schema = serde_json::to_value(
        Bucket::crd().spec.versions[0]
            .schema
            .as_ref()
            .unwrap()
            .open_api_v3_schema
            .as_ref()
            .unwrap(),
    )
    .unwrap();

    let props = schema.get("properties").unwrap().as_object().unwrap();
    assert!(props.contains_key("name"), "got {props:?}");
    assert!(props.contains_key("region"), "got {props:?}");
    assert!(!props.contains_key("spec"), "got {props:?}");
    assert_eq!(schema.get("required").unwrap(), &serde_json::json!(["name"]));
}

// `KubeSchema` is a separate schema derive from `JsonSchema`, so cover the flattened path there too.
#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, KubeSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Shard",
    namespaced,
    no_spec,
    derive = "PartialEq",
    validation = "has(self.replicas)"
)]
pub struct ShardFields {
    #[x_kube(validation = Rule::new("self >= 1").message("at least one replica"))]
    replicas: i32,
}

#[test]
fn no_spec_hoists_validations_onto_the_root_schema() {
    let schema = serde_json::to_value(
        Shard::crd().spec.versions[0]
            .schema
            .as_ref()
            .unwrap()
            .open_api_v3_schema
            .as_ref()
            .unwrap(),
    )
    .unwrap();

    assert_eq!(
        schema.get("x-kubernetes-validations").unwrap(),
        &serde_json::json!([{ "rule": "has(self.replicas)" }])
    );
    assert_eq!(
        schema
            .pointer("/properties/replicas/x-kubernetes-validations")
            .unwrap(),
        &serde_json::json!([{ "rule": "self >= 1", "message": "at least one replica" }])
    );
}
