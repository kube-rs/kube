use chrono::{DateTime, Utc};
use kube_derive::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
#[kube(apiextensions = "v1")]
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
}

fn default_value() -> String {
    "default_value".into()
}

fn default_nullable() -> Option<String> {
    Some("default_nullable".into())
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
fn test_crd_schema_matches_expected() {
    use kube::core::CustomResourceExt;
    assert_eq!(
        Foo::crd(),
        serde_json::from_value(serde_json::json!({
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
                                            "non_nullable": {
                                                "type": "string"
                                            },
                                            "non_nullable_with_default": {
                                                "default": "default_value",
                                                "type": "string"
                                            },

                                            "nullable_skipped": {
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullable": {
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullable_skipped_with_default": {
                                                "default": "default_nullable",
                                                "nullable": true,
                                                "type": "string"
                                            },
                                            "nullable_with_default": {
                                                "default": "default_nullable",
                                                "nullable": true,
                                                "type": "string"
                                            },

                                            "timestamp": {
                                                "type": "string",
                                                "format": "date-time"
                                            }
                                        },
                                        "required": [
                                            "non_nullable",
                                            "timestamp"
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
        }))
        .unwrap()
    );
}
