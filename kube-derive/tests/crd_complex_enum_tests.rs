#![allow(missing_docs)]
use assert_json_diff::assert_json_eq;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// A very simple enum with empty variants
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
enum NormalEnum {
    /// First variant
    A,
    /// Second variant
    B,
}

/// An untagged enum with a nested enum inside
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
enum UntaggedEnum {
    /// Used in case the `one` field of tpye [`u32`] is present
    A { one: String },
    /// Used in case the `two` field of type [`NormalEnum`] is present
    B { two: NormalEnum },
    /// Used in case no fields are present
    C {},
}

/// Put a [`UntaggedEnum`] behind `#[serde(flatten)]`,
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
struct FlattenedUntaggedEnum {
    #[serde(flatten)]
    inner: UntaggedEnum,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "NormalEnumTest")]
struct NormalEnumTestSpec {
    foo: NormalEnum,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "OptionalEnumTest")]
struct OptionalEnumTestSpec {
    foo: Option<NormalEnum>,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "UntaggedEnumTest")]
struct UntaggedEnumTestSpec {
    foo: UntaggedEnum,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "OptionalUntaggedEnumTest")]
struct OptionalUntaggedEnumTestSpec {
    foo: Option<UntaggedEnum>,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "FlattenedUntaggedEnumTest")]
struct FlattenedUntaggedEnumTestSpec {
    foo: FlattenedUntaggedEnum,
}

/// Use `cargo test --package kube-derive print_crds -- --nocapture` to get the CRDs as YAML.
/// Afterwards you can use `kubectl apply -f -` to see if they are valid CRDs.
#[test]
fn print_crds() {
    println!("{}", serde_yaml::to_string(&NormalEnumTest::crd()).unwrap());
    println!("---");
    println!("{}", serde_yaml::to_string(&OptionalEnumTest::crd()).unwrap());
    println!("---");
    println!("{}", serde_yaml::to_string(&UntaggedEnumTest::crd()).unwrap());
    println!("---");
    println!(
        "{}",
        serde_yaml::to_string(&OptionalUntaggedEnumTest::crd()).unwrap()
    );
    println!("---");
    println!(
        "{}",
        serde_yaml::to_string(&FlattenedUntaggedEnumTest::crd()).unwrap()
    );
}

#[test]
fn normal_enum() {
    assert_json_eq!(
        NormalEnumTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "normalenumtests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "NormalEnumTest",
                  "plural": "normalenumtests",
                  "shortNames": [],
                  "singular": "normalenumtest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for NormalEnumTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "description": "A very simple enum with empty variants",
                                "enum": [
                                  "A",
                                  "B"
                                ],
                                "type": "string"
                              }
                            },
                            "required": [
                              "foo"
                            ],
                            "type": "object"
                          }
                        },
                        "required": [
                          "spec"
                        ],
                        "title": "NormalEnumTest",
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
fn optional_enum() {
    assert_json_eq!(
        OptionalEnumTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "optionalenumtests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "OptionalEnumTest",
                  "plural": "optionalenumtests",
                  "shortNames": [],
                  "singular": "optionalenumtest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for OptionalEnumTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "anyOf": [
                                  {
                                    "description": "A very simple enum with empty variants",
                                    "enum": [
                                      "A",
                                      "B"
                                    ],
                                    "type": "string"
                                  },
                                  {
                                    "enum": [
                                      null
                                    ],
                                    "nullable": true
                                  }
                                ]
                              }
                            },
                            "type": "object"
                          }
                        },
                        "required": [
                          "spec"
                        ],
                        "title": "OptionalEnumTest",
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

    // The CustomResourceDefinition "optionalenumtests.clux.dev" is invalid:
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[0].description: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[0].type: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[1].nullable: Forbidden: must be false to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].type: Required value: must not be empty for specified object fields
    panic!("This CRD is currently not accepted by Kubernetes!");
}


#[test]
fn untagged_enum() {
    assert_json_eq!(
        UntaggedEnumTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "untaggedenumtests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "UntaggedEnumTest",
                  "plural": "untaggedenumtests",
                  "shortNames": [],
                  "singular": "untaggedenumtest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for UntaggedEnumTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "anyOf": [
                                  {
                                    "required": [
                                      "one"
                                    ]
                                  },
                                  {
                                    "required": [
                                      "two"
                                    ]
                                  },
                                  {
                                    "description": "Used in case no fields are present",
                                    "type": "object"
                                  }
                                ],
                                "description": "An untagged enum with a nested enum inside",
                                "properties": {
                                  "one": {
                                    "description": "Used in case the `one` field of tpye [`u32`] is present",
                                    "type": "string"
                                  },
                                  "two": {
                                    "description": "Used in case the `two` field of type [`NormalEnum`] is present",
                                    "enum": [
                                      "A",
                                      "B"
                                    ],
                                    "type": "string"
                                  }
                                },
                                "type": "object"
                              }
                            },
                            "required": [
                              "foo"
                            ],
                            "type": "object"
                          }
                        },
                        "required": [
                          "spec"
                        ],
                        "title": "UntaggedEnumTest",
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

    // The CustomResourceDefinition "untaggedenumtests.clux.dev" is invalid:
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[2].description: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[2].type: Forbidden: must be empty to be structural
    panic!("This CRD is currently not accepted by Kubernetes!");
}

#[test]
fn optional_untagged_enum() {
    assert_json_eq!(
        OptionalUntaggedEnumTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "optionaluntaggedenumtests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "OptionalUntaggedEnumTest",
                  "plural": "optionaluntaggedenumtests",
                  "shortNames": [],
                  "singular": "optionaluntaggedenumtest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for OptionalUntaggedEnumTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "anyOf": [
                                  {
                                    "anyOf": [
                                      {
                                        "required": [
                                          "one"
                                        ]
                                      },
                                      {
                                        "required": [
                                          "two"
                                        ]
                                      },
                                      {
                                        "description": "Used in case no fields are present",
                                        "type": "object"
                                      }
                                    ]
                                  },
                                  {
                                    "enum": [
                                      null
                                    ],
                                    "nullable": true
                                  }
                                ],
                                "properties": {
                                  "one": {
                                    "description": "Used in case the `one` field of tpye [`u32`] is present",
                                    "type": "string"
                                  },
                                  "two": {
                                    "description": "Used in case the `two` field of type [`NormalEnum`] is present",
                                    "enum": [
                                      "A",
                                      "B"
                                    ],
                                    "type": "string"
                                  }
                                },
                                "type": "object"
                              }
                            },
                            "type": "object"
                          }
                        },
                        "required": [
                          "spec"
                        ],
                        "title": "OptionalUntaggedEnumTest",
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

    // The CustomResourceDefinition "optionaluntaggedenumtests.clux.dev" is invalid:
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[0].anyOf[2].description: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[0].anyOf[2].type: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[1].nullable: Forbidden: must be false to be structural
    panic!("This CRD is currently not accepted by Kubernetes!");
}

#[test]
fn flattened_untagged_enum() {
    assert_json_eq!(
        FlattenedUntaggedEnumTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "flatteneduntaggedenumtests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "FlattenedUntaggedEnumTest",
                  "plural": "flatteneduntaggedenumtests",
                  "shortNames": [],
                  "singular": "flatteneduntaggedenumtest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for FlattenedUntaggedEnumTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "anyOf": [
                                  {
                                    "required": [
                                      "one"
                                    ]
                                  },
                                  {
                                    "required": [
                                      "two"
                                    ]
                                  },
                                  {
                                    "description": "Used in case no fields are present",
                                    "type": "object"
                                  }
                                ],
                                "description": "Put a [`UntaggedEnum`] behind `#[serde(flatten)]`,",
                                "properties": {
                                  "one": {
                                    "description": "Used in case the `one` field of tpye [`u32`] is present",
                                    "type": "string"
                                  },
                                  "two": {
                                    "description": "Used in case the `two` field of type [`NormalEnum`] is present",
                                    "enum": [
                                      "A",
                                      "B"
                                    ],
                                    "type": "string"
                                  }
                                },
                                "type": "object"
                              }
                            },
                            "required": [
                              "foo"
                            ],
                            "type": "object"
                          }
                        },
                        "required": [
                          "spec"
                        ],
                        "title": "FlattenedUntaggedEnumTest",
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

    // The CustomResourceDefinition "flatteneduntaggedenumtests.clux.dev" is invalid:
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[2].description: Forbidden: must be empty to be structural
    // * spec.validation.openAPIV3Schema.properties[spec].properties[foo].anyOf[2].type: Forbidden: must be empty to be structural
    panic!("This CRD is currently not accepted by Kubernetes!");
}
