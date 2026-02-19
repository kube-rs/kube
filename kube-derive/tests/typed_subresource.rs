#![allow(missing_docs)]

use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    status = "Status",
    printcolumn(json_path = ".spec.name", name = "Spec", type_ = "string"),
    scale(
        spec_replicas_path = ".spec.replicas",
        status_replicas_path = ".status.replicas",
        label_selector_path = ".spec.labelSelector"
    )
)]
struct FooSpec {}

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Compat",
    printcolumn = r#"{"jsonPath": ".spec.name", "name": "Spec", "type": "string"}"#,
    status = "Status",
    scale = r#"{"specReplicasPath": ".spec.replicas", "statusReplicasPath": ".status.replicas", "labelSelectorPath": ".spec.labelSelector"}"#
)]
struct CompatSpec {}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    replicas: usize,
    label_selector: String,
}

#[test]
fn test_printcolumns_backwards_compatible() {
    use kube::core::CustomResourceExt;
    // Check that the typed version of printcolumn matches the backwards compatible raw json string
    assert_eq!(
        Compat::crd().spec.versions[0].additional_printer_columns,
        Foo::crd().spec.versions[0].additional_printer_columns
    );
}

#[test]
fn test_scale_backwards_compatible() {
    use kube::core::CustomResourceExt;
    // Check that the typed version of scale matches the backwards compatible raw json string
    assert_eq!(
        Compat::crd().spec.versions[0]
            .subresources
            .as_ref()
            .map(|sr| sr.scale.clone()),
        Foo::crd().spec.versions[0]
            .subresources
            .as_ref()
            .map(|sr| sr.scale.clone())
    );
}
