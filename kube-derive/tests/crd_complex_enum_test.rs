#![allow(missing_docs)]

use assert_json_diff::assert_json_eq;
use kube::{CustomResource, CustomResourceExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

// Enum definitions

/// A very simple enum with unit variants
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
enum NormalEnum {
    /// First variant
    A,
    /// Second variant
    B,

    // No doc-comments on these variants
    C,
    D,
}

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum NormalEnumWithoutDescriptions {
    A,
    B,
    C,
    D,
}

/// A complex enum with tuple and struct variants
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
enum ComplexEnum {
    /// Override documentation on the Normal variant
    Normal(NormalEnum),

    /// Documentation on the Hardcore variant
    Hardcore {
        hard: String,
        core: NormalEnum,
        without_description: NormalEnumWithoutDescriptions,
    },
}

/// An untagged enum with a nested enum inside
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[serde(untagged)]
enum UntaggedEnum {
    /// Used in case the `one` field of type [`u32`] is present
    ///
    /// This should not appear in the schema because the "variant" disappears
    /// and this comment cannot pertain to all fields within the struct variant.
    A { one: String },
    /// Used in case the `two` field of type [`NormalEnum`] is present
    ///
    /// This should not appear in the schema because the "variant" disappears
    /// and this comment cannot pertain to all fields within the struct variant.
    B { two: NormalEnum, three: String },
    /// Used in case no fields are present
    ///
    /// This should not appear in the schema because the "variant" disappears
    /// and this comment cannot pertain to all fields within the struct variant.
    C {},
}

/// Put a [`UntaggedEnum`] behind `#[serde(flatten)]`
#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
struct FlattenedUntaggedEnum {
    #[serde(flatten)]
    inner: UntaggedEnum,
}

// CRD definitions

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
#[kube(
    group = "clux.dev",
    version = "v1",
    kind = "NormalEnumWithoutDescriptionsTest"
)]
struct NormalEnumWithoutDescriptionsTestSpec {
    foo: NormalEnumWithoutDescriptions,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "ComplexEnumTest")]
struct ComplexEnumTestSpec {
    foo: ComplexEnum,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "OptionalComplexEnumTest")]
struct OptionalComplexEnumTestSpec {
    /// Optional complex enum field
    // When this doc-comment is missing, we suggest using the doc-comment of the
    // inner type - though that is debatable
    foo: Option<ComplexEnum>,
}

#[derive(CustomResource, Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[kube(group = "clux.dev", version = "v1", kind = "FlattenedUntaggedEnumTest")]
struct FlattenedUntaggedEnumTestSpec {
    foo: FlattenedUntaggedEnum,
}

#[test]
fn complex_enum() {
    assert_json_eq!(
        ComplexEnumTest::crd(),
        json!(
          {
            "apiVersion": "apiextensions.k8s.io/v1",
            "kind": "CustomResourceDefinition",
            "metadata": {
              "name": "complexenumtests.clux.dev"
            },
            "spec": {
              "group": "clux.dev",
              "names": {
                "categories": [],
                "kind": "ComplexEnumTest",
                "plural": "complexenumtests",
                "shortNames": [],
                "singular": "complexenumtest"
              },
              "scope": "Cluster",
              "versions": [
                {
                  "additionalPrinterColumns": [],
                  "name": "v1",
                  "schema": {
                    "openAPIV3Schema": {
                      "description": "Auto-generated derived type for ComplexEnumTestSpec via `CustomResource`",
                      "properties": {
                        "spec": {
                          "properties": {
                            "foo": {
                              "description": "A complex enum with tuple and struct variants",
                              "oneOf": [
                                {
                                  "required": [
                                    "Normal"
                                  ]
                                },
                                {
                                  "required": [
                                    "Hardcore"
                                  ]
                                }
                              ],
                              "properties": {
                                "Hardcore": {
                                  "description": "Documentation on the Hardcore variant",
                                  "properties": {
                                    "core": {
                                      "description": "A very simple enum with unit variants",
                                      "enum": [
                                        "C",
                                        "D",
                                        "A",
                                        "B"
                                      ],
                                      "type": "string"
                                    },
                                    "hard": {
                                      "type": "string"
                                    },
                                    "without_description": {
                                      "enum": [
                                        "A",
                                        "B",
                                        "C",
                                        "D"
                                      ],
                                      "type": "string"
                                    }
                                  },
                                  "required": [
                                    "core",
                                    "hard",
                                    "without_description"
                                  ],
                                  "type": "object"
                                },
                                "Normal": {
                                  "description": "Override documentation on the Normal variant",
                                  "enum": [
                                    "C",
                                    "D",
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
                      "title": "ComplexEnumTest",
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
                              "description": "A very simple enum with unit variants",
                              "enum": [
                                "C",
                                "D",
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
                              "description": "A very simple enum with unit variants",
                              "enum": [
                                "C",
                                "D",
                                "A",
                                "B"
                              ],
                              "nullable": true,
                              "type": "string"
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
}

#[test]
fn normal_enum_without_descriptions() {
    assert_json_eq!(
        NormalEnumWithoutDescriptionsTest::crd(),
        json!(
            {
              "apiVersion": "apiextensions.k8s.io/v1",
              "kind": "CustomResourceDefinition",
              "metadata": {
                "name": "normalenumwithoutdescriptionstests.clux.dev"
              },
              "spec": {
                "group": "clux.dev",
                "names": {
                  "categories": [],
                  "kind": "NormalEnumWithoutDescriptionsTest",
                  "plural": "normalenumwithoutdescriptionstests",
                  "shortNames": [],
                  "singular": "normalenumwithoutdescriptionstest"
                },
                "scope": "Cluster",
                "versions": [
                  {
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                      "openAPIV3Schema": {
                        "description": "Auto-generated derived type for NormalEnumWithoutDescriptionsTestSpec via `CustomResource`",
                        "properties": {
                          "spec": {
                            "properties": {
                              "foo": {
                                "enum": [
                                  "A",
                                  "B",
                                  "C",
                                  "D"
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
                        "title": "NormalEnumWithoutDescriptionsTest",
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
fn optional_complex_enum() {
    assert_json_eq!(
        OptionalComplexEnumTest::crd(),
        json!({
            "apiVersion": "apiextensions.k8s.io/v1",
            "kind": "CustomResourceDefinition",
            "metadata": {
                "name": "optionalcomplexenumtests.clux.dev"
            },
            "spec": {
                "group": "clux.dev",
                "names": {
                    "categories": [],
                    "kind": "OptionalComplexEnumTest",
                    "plural": "optionalcomplexenumtests",
                    "shortNames": [],
                    "singular": "optionalcomplexenumtest"
                },
                "scope": "Cluster",
                "versions": [{
                    "additionalPrinterColumns": [],
                    "name": "v1",
                    "schema": {
                        "openAPIV3Schema": {
                            "description": "Auto-generated derived type for OptionalComplexEnumTestSpec via `CustomResource`",
                            "properties": {
                                "spec": {
                                    "properties": {
                                        "foo": {
                                            "description": "Optional complex enum field",
                                            "nullable": true,
                                            "oneOf": [
                                                {
                                                    "required": ["Normal"]
                                                },
                                                {
                                                    "required": ["Hardcore"]
                                                }
                                            ],
                                            "properties": {
                                                "Hardcore": {
                                                    "description": "Documentation on the Hardcore variant",
                                                    "properties": {
                                                        "core": {
                                                            "description": "A very simple enum with unit variants",
                                                            "enum": ["C", "D", "A", "B"],
                                                            "type": "string"
                                                        },
                                                        "hard": {
                                                            "type": "string"
                                                        },
                                                        "without_description": {
                                                            "enum": ["A", "B", "C", "D"],
                                                            "type": "string"
                                                        }
                                                    },
                                                    "required": ["core", "hard", "without_description"],
                                                    "type": "object"
                                                },
                                                "Normal": {
                                                    "description": "Override documentation on the Normal variant",
                                                    "enum": ["C", "D", "A", "B"],
                                                    "type": "string"
                                                }
                                            },
                                            "type": "object"
                                        }
                                    },
                                    "type": "object"
                                }
                            },
                            "required": ["spec"],
                            "title": "OptionalComplexEnumTest",
                            "type": "object"
                        }
                    },
                    "served": true,
                    "storage": true,
                    "subresources": {}
                }]
            }
        })
    );
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
                                    "three",
                                    "two"
                                  ]
                                },
                                {}
                              ],
                              "description": "Put a [`UntaggedEnum`] behind `#[serde(flatten)]`",
                              "properties": {
                                "one": {
                                  "type": "string"
                                },
                                "two": {
                                  "description": "A very simple enum with unit variants",
                                  "enum": [
                                    "C",
                                    "D",
                                    "A",
                                    "B"
                                  ],
                                  "type": "string"
                                },
                                "three": {
                                  "type": "string"
                                },
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
}
