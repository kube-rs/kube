#![allow(missing_docs)]
#![recursion_limit = "256"]

use assert_json_diff::assert_json_eq;
use jiff::Timestamp;
use kube::KubeSchema;
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// See `crd_derive_schema` example for how the schema generated from this struct affects defaulting and validation.
#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, KubeSchema)]
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "Foo",
    category = "clux",
    namespaced,
    doc = "Custom resource representing a Foo",
    derive = "PartialEq",
    shortname = "fo",
    shortname = "f",
    served = false,
    storage = false,
    deprecated = "my warning",
    selectable = ".spec.nonNullable",
    selectable = ".spec.nullable",
    annotation("clux.dev", "cluxingv1"),
    annotation("clux.dev/firewall", "enabled"),
    label("clux.dev", "cluxingv1"),
    label("clux.dev/persistence", "disabled"),
    validation = "self.metadata.name == 'singleton'",
    status = "Status",
    printcolumn(json_path = ".spec.name", name = "Spec", type_ = "string"),
    scale(
        spec_replicas_path = ".spec.replicas",
        status_replicas_path = ".status.replicas",
        label_selector_path = ".status.labelSelector"
    )
)]
#[x_kube(validation = "has(self.nonNullable)")]
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

    // Using feature `jiff`
    timestamp: Timestamp,

    /// This is a complex enum with a description
    #[x_kube(validation = "!has(self.variantOne) || self.variantOne.int > 22")]
    complex_enum: ComplexEnum,

    /// This is a untagged enum with a description
    untagged_enum_person: UntaggedEnumPerson,

    #[validate(length(min = 1))]
    my_list: Vec<String>,

    set: HashSet<String>,

    #[serde(default = "FooSpec::default_value")]
    associated_default: bool,

    #[x_kube(merge_strategy = ListMerge::Set)]
    x_kubernetes_set: Vec<String>,

    optional_enum: Option<Gender>,

    /// Preferred gender
    optional_enum_with_doc: Option<Gender>,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    replicas: usize,
    label_selector: String,
}

fn default_value() -> String {
    "default_value".into()
}

fn default_nullable() -> Option<String> {
    Some("default_nullable".into())
}

impl FooSpec {
    fn default_value() -> bool {
        true
    }
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
            timestamp: Timestamp::new(0, 0).unwrap(),
            complex_enum: ComplexEnum::VariantOne { int: 23 },
            untagged_enum_person: UntaggedEnumPerson::GenderAndAge(GenderAndAge {
                age: 42,
                gender: Gender::Male,
            }),
            associated_default: false,
            my_list: vec!["".into()],
            set: HashSet::from(["foo".to_owned()]),
            x_kubernetes_set: vec![],
            optional_enum: Some(Gender::Other),
            optional_enum_with_doc: Some(Gender::Other),
        }))
        .unwrap(),
        serde_json::json!({
            "apiVersion": "clux.dev/v1",
            "kind": "Foo",
            "metadata": {
                "annotations": {
                    "clux.dev": "cluxingv1",
                    "clux.dev/firewall": "enabled",
                },
                "labels": {
                    "clux.dev": "cluxingv1",
                    "clux.dev/persistence": "disabled",
                },
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
                },
                "associatedDefault": false,
                "myList": [""],
                "set": ["foo"],
                "xKubernetesSet": [],
                "optionalEnum": "Other",
                "optionalEnumWithDoc": "Other",
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
                "annotations": {
                    "clux.dev": "cluxingv1",
                    "clux.dev/firewall": "enabled",
                },
                "labels": {
                    "clux.dev": "cluxingv1",
                    "clux.dev/persistence": "disabled",
                },
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
                        "served": false,
                        "storage": false,
                        "deprecated": true,
                        "deprecationWarning": "my warning",
                        "additionalPrinterColumns": [{
                            "jsonPath": ".spec.name",
                            "name": "Spec",
                            "type": "string",
                        }],
                        "selectableFields": [{
                            "jsonPath": ".spec.nonNullable"
                        }, {
                            "jsonPath": ".spec.nullable"
                        }],
                        "subresources": {
                            "status": {},
                            "scale": {
                                "specReplicasPath": ".spec.replicas",
                                "labelSelectorPath": ".status.labelSelector",
                                "statusReplicasPath": ".status.replicas"
                            }
                        },
                        "schema": {
                            "openAPIV3Schema": {
                                "description": "Custom resource representing a Foo",
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
                                                "x-kubernetes-validations": [{
                                                    "rule": "!has(self.variantOne) || self.variantOne.int > 22",
                                                }],
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
                                            },
                                            "myList": {
                                                "type": "array",
                                                "items": {
                                                    "type": "string"
                                                },
                                                "minItems": 1,
                                            },
                                            "set": {
                                                "type": "array",
                                                "items": {
                                                    "type": "string"
                                                },
                                            },
                                            "associatedDefault": {
                                                "type": "boolean",
                                                "default": true,
                                            },
                                            "xKubernetesSet": {
                                                "type": "array",
                                                "items": {
                                                    "type": "string"
                                                },
                                                "x-kubernetes-list-type": "set",
                                            },
                                            "optionalEnum": {
                                                "nullable": true,
                                                "type": "string",
                                                "enum": [
                                                    "Female",
                                                    "Male",
                                                    "Other"
                                                ],
                                            },
                                            "optionalEnumWithDoc": {
                                                "description": "Preferred gender",
                                                "nullable": true,
                                                "type": "string",
                                                "enum": [
                                                    "Female",
                                                    "Male",
                                                    "Other"
                                                ],
                                            }
                                        },
                                        "required": [
                                            "complexEnum",
                                            "myList",
                                            "nonNullable",
                                            "set",
                                            "timestamp",
                                            "untaggedEnumPerson",
                                            "xKubernetesSet"
                                        ],
                                        "x-kubernetes-validations": [{
                                            "rule": "has(self.nonNullable)",
                                        }],
                                        "type": "object"
                                    },
                                    "status": {
                                        "properties": {
                                            "replicas": {
                                                "type": "integer",
                                                "format": "uint",
                                                "minimum": 0.0,
                                            },
                                            "labelSelector": {
                                                "type": "string"
                                            }
                                        },
                                        "required": [
                                            "labelSelector",
                                            "replicas"
                                        ],
                                        "nullable": true,
                                        "type": "object"
                                    }
                                },
                                "required": [
                                    "spec"
                                ],
                                "x-kubernetes-validations": [{
                                    "rule": "self.metadata.name == 'singleton'",
                                }],
                                "title": "FooValidated",
                                "type": "object"
                            }
                        },
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

// Test for Option<IntOrString> nullable handling (issue #1869)
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "IntOrStringTest")]
pub struct IntOrStringTestSpec {
    pub required_int_or_string: k8s_openapi::apimachinery::pkg::util::intstr::IntOrString,
    pub optional_int_or_string: Option<k8s_openapi::apimachinery::pkg::util::intstr::IntOrString>,
}

// Test for deny_unknown_fields handling (issue #1828)
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "DenyUnknown")]
pub struct DenyUnknownSpec {
    pub subitem: SubItemDenyUnknown,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SubItemDenyUnknown {
    pub one: String,
    pub two: bool,
    pub three: i32,
}

#[test]
fn deny_unknown_fields() {
    use kube::core::CustomResourceExt;
    let crd = DenyUnknown::crd();
    let spec_schema = &crd.spec.versions[0]
        .schema
        .as_ref()
        .unwrap()
        .open_api_v3_schema
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap()["spec"];

    let subitem_schema = &spec_schema.properties.as_ref().unwrap()["subitem"];
    assert!(subitem_schema.additional_properties.is_none());
}

#[test]
fn test_optional_int_or_string_nullable() {
    use kube::core::CustomResourceExt;
    let crd = IntOrStringTest::crd();
    let spec_schema = &crd.spec.versions[0]
        .schema
        .as_ref()
        .unwrap()
        .open_api_v3_schema
        .as_ref()
        .unwrap()
        .properties
        .as_ref()
        .unwrap()["spec"];

    let properties = spec_schema.properties.as_ref().unwrap();

    // Required field should have x-kubernetes-int-or-string but not nullable
    let required = &properties["required_int_or_string"];
    assert_eq!(required.x_kubernetes_int_or_string, Some(true));
    assert_eq!(required.nullable, None);

    // Optional field should have both x-kubernetes-int-or-string and nullable
    let optional = &properties["optional_int_or_string"];
    assert_eq!(optional.x_kubernetes_int_or_string, Some(true));
    assert_eq!(optional.nullable, Some(true));
}
