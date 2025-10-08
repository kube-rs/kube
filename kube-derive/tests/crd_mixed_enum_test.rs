#![allow(missing_docs)]
use assert_json_diff::assert_json_eq;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// This enum is invalid, as "plain" (string) variants are mixed with object variants
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "InvalidEnum1")]
enum InvalidEnum1Spec {
    /// Unit variant (represented as string)
    A,
    /// Takes an [`u32`] (represented as object)
    B(u32),
}

/// This enum is invalid, as "plain" (string) variants are mixed with object variants
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "InvalidEnum2")]
enum InvalidEnum2Spec {
    /// Unit variant (represented as string)
    A,
    /// Takes a single field (represented as object)
    B { inner: u32 },
}

/// This enum is valid, as all variants are objects
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "ValidEnum3")]
enum ValidEnum3Spec {
    /// Takes an [`String`] (represented as object)
    A(String),
    /// Takes an [`u32`] (represented as object)
    B(u32),
}

// This enum intentionally has no documentation to increase test coverage!
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "ValidEnum4")]
enum ValidEnum4Spec {
    A(String),
    B { inner: u32 },
}

/// Use `cargo test --package kube-derive print_crds -- --nocapture` to get the CRDs as YAML.
/// Afterwards you can use `kubectl apply -f -` to see if they are valid CRDs.
#[test]
fn print_crds() {
    println!("{}", serde_yaml::to_string(&ValidEnum3::crd()).unwrap());
    println!("---");
    println!("{}", serde_yaml::to_string(&ValidEnum4::crd()).unwrap());
}

#[test]
#[should_panic = "Enum variant set [String(\"A\")] has type Single(String) but was already defined as Some(Single(Object)). The instance type must be equal for all subschema variants."]
fn invalid_enum_1() {
    InvalidEnum1::crd();
}

#[test]
#[should_panic = "Enum variant set [String(\"A\")] has type Single(String) but was already defined as Some(Single(Object)). The instance type must be equal for all subschema variants."]
fn invalid_enum_2() {
    InvalidEnum2::crd();
}

#[test]
fn valid_enum_3() {
    assert_json_eq!(
        ValidEnum3::crd(),
        json!(
          {
            "apiVersion": "apiextensions.k8s.io/v1",
            "kind": "CustomResourceDefinition",
            "metadata": {
              "name": "validenum3s.clux.dev"
            },
            "spec": {
              "group": "clux.dev",
              "names": {
                "categories": [],
                "kind": "ValidEnum3",
                "plural": "validenum3s",
                "shortNames": [],
                "singular": "validenum3"
              },
              "scope": "Cluster",
              "versions": [
                {
                  "additionalPrinterColumns": [],
                  "name": "v1",
                  "schema": {
                    "openAPIV3Schema": {
                      "description": "Auto-generated derived type for ValidEnum3Spec via `CustomResource`",
                      "properties": {
                        "spec": {
                          "description": "This enum is valid, as all variants are objects",
                          "oneOf": [
                            {
                              "required": [
                                "A"
                              ]
                            },
                            {
                              "required": [
                                "B"
                              ]
                            }
                          ],
                          "properties": {
                            "A": {
                              "description": "Takes an [`String`] (represented as object)",
                              "type": "string"
                            },
                            "B": {
                              "description": "Takes an [`u32`] (represented as object)",
                              "format": "uint32",
                              "minimum": 0.0,
                              "type": "integer"
                            }
                          },
                          "type": "object"
                        }
                      },
                      "required": [
                        "spec"
                      ],
                      "title": "ValidEnum3",
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
        )
    );
}

#[test]
fn valid_enum_4() {
    assert_json_eq!(
        ValidEnum4::crd(),
        json!(
          {
            "apiVersion": "apiextensions.k8s.io/v1",
            "kind": "CustomResourceDefinition",
            "metadata": {
              "name": "validenum4s.clux.dev"
            },
            "spec": {
              "group": "clux.dev",
              "names": {
                "categories": [],
                "kind": "ValidEnum4",
                "plural": "validenum4s",
                "shortNames": [],
                "singular": "validenum4"
              },
              "scope": "Cluster",
              "versions": [
                {
                  "additionalPrinterColumns": [],
                  "name": "v1",
                  "schema": {
                    "openAPIV3Schema": {
                      "description": "Auto-generated derived type for ValidEnum4Spec via `CustomResource`",
                      "properties": {
                        "spec": {
                          "oneOf": [
                            {
                              "required": [
                                "A"
                              ]
                            },
                            {
                              "required": [
                                "B"
                              ]
                            }
                          ],
                          "properties": {
                            "A": {
                              "type": "string"
                            },
                            "B": {
                              "properties": {
                                "inner": {
                                  "format": "uint32",
                                  "minimum": 0.0,
                                  "type": "integer"
                                }
                              },
                              "required": [
                                "inner"
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
                      "title": "ValidEnum4",
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
        )
    );
}

#[test]
#[should_panic = "Enum variant set [String(\"A\")] has type Single(String) but was already defined as Some(Single(Object)). The instance type must be equal for all subschema variants."]
fn struct_with_enum_1() {
    #[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
    #[kube(group = "clux.dev", version = "v1", kind = "Foo")]
    struct FooSpec {
        foo: InvalidEnum1,
    }

    Foo::crd();
}
