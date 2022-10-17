#![recursion_limit = "256"]

use assert_json_diff::assert_json_eq;
use chrono::{DateTime, NaiveDateTime, Utc};
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// See `crd_derive_schema` example for how the schema generated from this struct affects defaulting and validation.
#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    category = "clux",
    namespaced,
    derive = "PartialEq",
    shortname = "fo",
    shortname = "f"
)]
#[serde(rename_all = "camelCase")]
struct FooSpec {
    non_nullable: String,

    #[serde(default = "default_value")]
    non_nullable_with_default: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    nullable_skipped: Option<String>,
    nullable: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default = "default_nullable")]
    nullable_skipped_with_default: Option<String>,

    #[serde(default = "default_nullable")]
    nullable_with_default: Option<String>,

    // Using feature `chrono`
    timestamp: DateTime<Utc>,

    /// This is a complex enum with a description
    complex_enum: ComplexEnum,

    /// This is a untagged enum with a description
    untagged_enum_person: UntaggedEnumPerson,
}

fn default_value() -> String {
    "default_value".into()
}

fn default_nullable() -> Option<String> {
    Some("default_nullable".into())
}

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "Flattening")]
pub struct FlatteningSpec {
    foo: String,
    #[serde(flatten)]
    arbitrary: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[allow(clippy::enum_variant_names)]
enum ComplexEnum {
    /// First variant with an int
    VariantOne { int: i32 },
    /// Second variant with an String
    VariantTwo { str: String },
    /// Third variant which doesn't has an attribute
    VariantThree {},
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[serde(untagged)]
enum UntaggedEnumPerson {
    GenderAndAge(GenderAndAge),
    GenderAndDateOfBirth(GenderAndDateOfBirth),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GenderAndAge {
    /// Gender of the person
    gender: Gender,
    /// Age of the person in years
    age: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GenderAndDateOfBirth {
    /// Gender of the person
    gender: Gender,
    /// Date of birth of the person as ISO 8601 date
    date_of_birth: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[serde(rename_all = "PascalCase")]
enum Gender {
    Female,
    Male,
    /// This variant has a comment!
    Other,
}

#[test]
fn test_crd_name() {
    use kube::core::CustomResourceExt;
    assert_eq!("foos.clux.dev", Foo::crd_name());
}

#[test]
fn test_shortnames() {
    use kube::core::CustomResourceExt;
    assert_eq!(&["fo", "f"], Foo::shortnames());
}

#[test]
fn test_serialized_matches_expected() {
    assert_json_eq!(
        serde_json::to_value(Foo::new("bar", FooSpec {
            non_nullable: "asdf".to_string(),
            non_nullable_with_default: "asdf".to_string(),
            nullable_skipped: None,
            nullable: None,
            nullable_skipped_with_default: None,
            nullable_with_default: None,
            timestamp: DateTime::from_utc(NaiveDateTime::from_timestamp(0, 0), Utc),
            complex_enum: ComplexEnum::VariantOne { int: 23 },
            untagged_enum_person: UntaggedEnumPerson::GenderAndAge(GenderAndAge {
                age: 42,
                gender: Gender::Male,
            })
        }))
        .unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "Foo",
            "metadata": {
                "name": "bar",
            },
            "spec": {
                "nonNullable": "asdf",
                "nonNullableWithDefault": "asdf",
                "nullable": null,
                "nullableWithDefault": null,
                "timestamp": "1970-01-01T00:00:00Z",
                "complexEnum": {
                    "variantOne": {
                        "int": 23
                    }
                },
                "untaggedEnumPerson": {
                    "age": 42,
                    "gender": "Male"
                }
            }
        })
    )
}

#[test]
fn test_crd_schema_matches_expected() {
    use kube::core::CustomResourceExt;
    assert_json_eq!(
        Foo::crd(),
        serde_json::json!({
            "apiVersion": "apiextensions.k8s.io/v1",
            "kind": "CustomResourceDefinition",
            "metadata": {
                "name": "foos.clux.dev"
            },
            "spec": {
                "group": "clux.dev",
                "names": {
                    "categories": ["clux"],
                    "kind": "Foo",
                    "plural": "foos",
                    "shortNames": ["fo", "f"],
                    "singular": "foo"
                },
                "scope": "Namespaced",
                "versions": [
                    {
                        "name": "v1",
                        "served": true,
                        "storage": true,
                        "additionalPrinterColumns": [],
                        "schema": {
                            "openAPIV3Schema": {
                                "description": "Auto-generated derived type for FooSpec via `CustomResource`",
                                "properties": {
                                    "spec": {
                                        "properties": {
                                            "nonNullable": {
                                                "type": "string"
                                            },
                                            "nonNullableWithDefault": {
                                                "default": "default_value",
                                                "type": "string"
                                            },

                                            "nullableSkipped": {
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullable": {
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullableSkippedWithDefault": {
                                                "default": "default_nullable",
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullableWithDefault": {
                                                "default": "default_nullable",
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "timestamp": {
                                                "type": "string",
                                                "format": "date-time"
                                            },
                                            "complexEnum": {
                                                "type": "object",
                                                "properties": {
                                                    "variantOne": {
                                                        "type": "object",
                                                        "properties": {
                                                            "int": {
                                                                "type": "integer",
                                                                "format": "int32"
                                                            }
                                                        },
                                                        "required": ["int"],
                                                        "description": "First variant with an int"
                                                    },
                                                    "variantTwo": {
                                                        "type": "object",
                                                        "properties": {
                                                            "str": {
                                                                "type": "string"
                                                            }
                                                        },
                                                        "required": ["str"],
                                                        "description": "Second variant with an String"
                                                    },
                                                    "variantThree": {
                                                        "type": "object",
                                                        "description": "Third variant which doesn't has an attribute"
                                                    }
                                                },
                                                "oneOf": [
                                                    {
                                                        "required": ["variantOne"]
                                                    },
                                                    {
                                                        "required": ["variantTwo"]
                                                    },
                                                    {
                                                        "required": ["variantThree"]
                                                    }
                                                ],
                                                "description": "This is a complex enum with a description"
                                            },
                                            "untaggedEnumPerson": {
                                                "type": "object",
                                                "properties": {
                                                    "age": {
                                                        "type": "integer",
                                                        "format": "int32",
                                                        "description": "Age of the person in years"
                                                    },
                                                    "dateOfBirth": {
                                                        "type": "string",
                                                        "description": "Date of birth of the person as ISO 8601 date"
                                                    },
                                                    "gender": {
                                                        "type": "string",
                                                        "enum": ["Female", "Male", "Other"],
                                                        "description": "Gender of the person"
                                                    }
                                                },
                                                "anyOf": [
                                                    {
                                                        "required": ["age", "gender"]
                                                    },
                                                    {
                                                        "required": ["dateOfBirth", "gender"]
                                                    }
                                                ],
                                                "description": "This is a untagged enum with a description"
                                            }
                                        },
                                        "required": [
                                            "complexEnum",
                                            "nonNullable",
                                            "timestamp",
                                            "untaggedEnumPerson"
                                        ],
                                        "type": "object"
                                    }
                                },
                                "required": [
                                    "spec"
                                ],
                                "title": "Foo",
                                "type": "object"
                            }
                        },
                        "subresources": {},
                    }
                ]
            }
        })
    );
}

#[test]
fn flattening() {
    use kube::core::CustomResourceExt;
    let spec = &Flattening::crd().spec.versions[0]
        .schema
        .clone()
        .unwrap()
        .open_api_v3_schema
        .unwrap()
        .properties
        .unwrap()["spec"];
    assert_eq!(spec.x_kubernetes_preserve_unknown_fields, Some(true));
    assert_eq!(spec.additional_properties, None);
}
