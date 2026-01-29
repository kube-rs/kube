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

/// This enum is invalid, as the types of the `inner` fields differ.
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "InvalidEnum5")]
#[serde(untagged)]
enum InvalidEnum5Spec {
    A { inner: String },
    B { inner: u32 },
}

/// This enum is valid, as the `inner` fields are the same.
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "ValidEnum6")]
#[serde(untagged)]
enum ValidEnum6Spec {
    A {
        /// The inner fields need to have the same schema (so also same description)
        inner: u32,
        /// An additional field
        additional: String,
    },
    B {
        /// The inner fields need to have the same schema (so also same description)
        inner: u32,
    },
}

/// This enum is invalid, as the typed of `inner` fields are the same, *but* the description (which
/// is part of the schema differs).
#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "InvalidEnum7")]
#[serde(untagged)]
enum InvalidEnum7Spec {
    A {
        /// The inner fields need to have the same schema (so also same description)
        inner: u32,
        additional: String,
    },
    B {
        /// This description differs!
        inner: u32,
    },
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
#[should_panic = "Property \"inner\" has the schema Object(SchemaObject { metadata: None, instance_type: Some(Single(Integer)), format: Some(\"uint32\"), enum_values: None, subschemas: None, array: None, object: None, extensions: {\"minimum\": Number(0)}, other: Object {\"minimum\": Number(0)} }) but was already defined as Object(SchemaObject { metadata: None, instance_type: Some(Single(String)), format: None, enum_values: None, subschemas: None, array: None, object: None, extensions: {}, other: Object {} }) in another subschema. The schemas for a property used in multiple subschemas must be identical"]
fn invalid_enum_5() {
    InvalidEnum5::crd();
}

#[test]
fn valid_enum_6() {
    assert_json_eq!(
        ValidEnum6::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "validenum6s.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "ValidEnum6",
                  "plural": "validenum6s",
                  "shortNames": [],
                  "singular": "validenum6"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for ValidEnum6Spec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "anyOf": [
                              {
                                "required": [
                                  "additional",
                                  "inner"
                                ]
                              },
                              {
                                "required": [
                                  "inner"
                                ]
                              }
                            ],
                            "description": "This enum is valid, as the `inner` fields are the same.",
                            "properties": {
                              "additional": {
                                "description": "An additional field",
                                "type": "string"
                              },
                              "inner": {
                                "description": "The inner fields need to have the same schema (so also same description)",
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
                        "title": "ValidEnum6",
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
#[should_panic = "Property \"inner\" has the schema Object(SchemaObject { metadata: Some(Metadata { description: Some(\"This description differs!\"), default: None }), instance_type: Some(Single(Integer)), format: Some(\"uint32\"), enum_values: None, subschemas: None, array: None, object: None, extensions: {\"minimum\": Number(0)}, other: Object {\"minimum\": Number(0)} }) but was already defined as Object(SchemaObject { metadata: Some(Metadata { description: Some(\"The inner fields need to have the same schema (so also same description)\"), default: None }), instance_type: Some(Single(Integer)), format: Some(\"uint32\"), enum_values: None, subschemas: None, array: None, object: None, extensions: {\"minimum\": Number(0)}, other: Object {\"minimum\": Number(0)} }) in another subschema. The schemas for a property used in multiple subschemas must be identical"]
fn invalid_enum_7() {
    InvalidEnum7::crd();
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
