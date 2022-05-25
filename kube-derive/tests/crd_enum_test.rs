use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "FooEnum")]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
enum FooEnumSpec {
    /// First variant with an int
    VariantOne { int: i32 },
    /// Second variant with an String
    VariantTwo { str: String },
    /// Third variant which doesn't has an attribute
    VariantThree {},
}

#[test]
fn test_crd_name() {
    use kube::core::CustomResourceExt;
    assert_eq!("fooenums.clux.dev", FooEnum::crd_name());
}

#[test]
fn test_serialized_matches_expected() {
    assert_eq!(
        serde_json::to_value(FooEnum::new("bar", FooEnumSpec::VariantOne { int: 42 })).unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "FooEnum",
            "metadata": {
                "name": "bar",
            },
            "spec": {
                "variantOne": {
                    "int": 42
                }
            }
        })
    );
    assert_eq!(
        serde_json::to_value(FooEnum::new("bar", FooEnumSpec::VariantThree {})).unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "FooEnum",
            "metadata": {
                "name": "bar",
            },
            "spec": {
                "variantThree": {}
            }
        })
    );
}

#[test]
fn test_crd_schema_matches_expected() {
    use kube::core::CustomResourceExt;

    assert_eq!(
        FooEnum::crd(),
        serde_json::from_value(serde_json::json!({
          "apiVersion": "apiextensions.k8s.io/v1",
          "kind": "CustomResourceDefinition",
          "metadata": {
            "name": "fooenums.clux.dev"
          },
          "spec": {
            "group": "clux.dev",
            "names": {
              "categories": [],
              "kind": "FooEnum",
              "plural": "fooenums",
              "shortNames": [],
              "singular": "fooenum"
            },
            "scope": "Cluster",
            "versions": [
              {
                "additionalPrinterColumns": [],
                "name": "v1",
                "schema": {
                  "openAPIV3Schema": {
                    "description": "Auto-generated derived type for FooEnumSpec via `CustomResource`",
                    "properties": {
                      "spec": {
                        "oneOf": [
                          {
                            "required": [
                              "variantOne"
                            ]
                          },
                          {
                            "required": [
                              "variantTwo"
                            ]
                          },
                          {
                            "required": [
                              "variantThree"
                            ]
                          }
                        ],
                        "properties": {
                          "variantOne": {
                            "description": "First variant with an int",
                            "properties": {
                              "int": {
                                "format": "int32",
                                "type": "integer"
                              }
                            },
                            "required": [
                              "int"
                            ],
                            "type": "object"
                          },
                          "variantThree": {
                            "description": "Third variant which doesn't has an attribute",
                            "type": "object"
                          },
                          "variantTwo": {
                            "description": "Second variant with an String",
                            "properties": {
                              "str": {
                                "type": "string"
                              }
                            },
                            "required": [
                              "str"
                            ],
                            "type": "object"
                          }
                        },
                        "type": "object"
                      }
                    },
                    "required": [
                      "spec"
                    ],
                    "title": "FooEnum",
                    "type": "object"
                  }
                },
                "served": true,
                "storage": true,
                "subresources": {}
              }
            ]
          }
        }
        ))
        .unwrap()
    );
}
