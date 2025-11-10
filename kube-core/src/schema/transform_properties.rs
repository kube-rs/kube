use crate::schema::{InstanceType, Metadata, Schema, SchemaObject, SingleOrVec, NULL_SCHEMA};

/// Take oneOf or anyOf subschema properties and move them them into the schema
/// properties.
///
/// Used for correcting the schema for serde untagged structural enums.
/// NOTE: Doc-comments are not preserved for untagged enums.
///
/// This will return early without modifications unless:
/// - There are `oneOf` or `anyOf` subschemas
/// - Each subschema has the type "object"
///
/// NOTE: This should work regardless of whether other hoisting has been performed or not.
pub(crate) fn hoist_properties_for_any_of_subschemas(kube_schema: &mut SchemaObject) {
    // Run some initial checks in case there is nothing to do
    let SchemaObject {
        subschemas: Some(subschemas),
        object: parent_object,
        ..
    } = kube_schema
    else {
        return;
    };

    // Deal with both tagged and untagged enums.
    // Untagged enum descriptions will not be preserved.
    let (subschemas, preserve_description) = match (subschemas.any_of.as_mut(), subschemas.one_of.as_mut()) {
        (None, None) => return,
        (None, Some(one_of)) => (one_of, true),
        (Some(any_of), None) => (any_of, false),
        (Some(_), Some(_)) => panic!("oneOf and anyOf are mutually exclusive"),
    };

    if subschemas.is_empty() {
        return;
    }

    // Ensure we aren't looking at the subschema with a null, as that is hoisted by another
    // transformer.
    if subschemas.len() == 2 {
        // Return if there is a null entry
        if subschemas
            .iter()
            .map(|schema| serde_json::to_value(schema).expect("schema should be able to convert to JSON"))
            .any(|value| value == *NULL_SCHEMA)
        {
            return;
        }
    }

    // At this point, we can be reasonably sure we need to operate on the schema.
    // TODO (@NickLarsenNZ): Return errors instead of panicking, leave panicking up to the
    // infallible schemars::Transform

    let subschemas = subschemas
        .iter_mut()
        .map(|schema| match schema {
            Schema::Object(schema_object) => schema_object,
            Schema::Bool(_) => panic!("oneOf and anyOf variants cannot be of type boolean"),
        })
        .collect::<Vec<_>>();

    for subschema in subschemas {
        // Drop the "type" field on subschema. It needs to be set to "object" on the schema.
        subschema.instance_type.take();
        kube_schema.instance_type = Some(SingleOrVec::Single(Box::new(InstanceType::Object)));

        // Take the description (which will be preserved for tagged enums further down).
        // This (along with the dropping of the "type" above) will allow for empty variants ({}).
        let subschema_metadata = subschema.metadata.take();

        if let Some(object) = subschema.object.as_deref_mut() {
            // Kubernetes doesn't allow variants to set additionalProperties
            object.additional_properties.take();

            // For a tagged enum (oneOf), we need to preserve the variant description
            if preserve_description {
                if let Some(Schema::Object(property_schema)) = object.properties.values_mut().next() {
                    if let Some(Metadata {
                        description: Some(_), ..
                    }) = subschema_metadata.as_deref()
                    {
                        property_schema.metadata = subschema_metadata
                    }
                };
            }

            // If subschema properties are set, hoist them to the schema properties.
            // This will panic if duplicate properties are encountered that do not have the same
            // shape. That can happen when the untagged enum variants each refer to structs which
            // contain the same field name but with different types or doc-comments.
            // The developer needs to make them the same.
            while let Some((property_name, Schema::Object(property_schema_object))) =
                object.properties.pop_first()
            {
                let parent_object = parent_object
                    // get the `ObjectValidation`, or an empty one without any properties set
                    .get_or_insert_default();

                // This would check that the variant property (that we want to now hoist)
                // is exactly the same as what is already hoisted (in this function).
                if let Some(existing_property) = parent_object.properties.get(&property_name) {
                    // TODO (@NickLarsenNZ): Here we could do another check to see if it only
                    // differs by description. If the schema property description is not set, then
                    // we could overwrite it and not panic.
                    assert_eq!(
                        existing_property,
                        &Schema::Object(property_schema_object.clone()),
                        "Properties for {property_name:?} are defined multiple times with different shapes"
                    );
                } else {
                    // Otherwise, insert the subschema properties into the schema properties
                    parent_object
                        .properties
                        .insert(property_name.clone(), Schema::Object(property_schema_object));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;

    use super::*;

    #[test]
    fn tagged_enum_with_stuct_and_tuple_variants_before_one_of_hoisting() {
        let original_schema_object_value = serde_json::json!({
            "description": "A complex tagged enum with unit and struct variants",
            "oneOf": [
                {
                    "additionalProperties": false,
                    "description": "Override documentation on the Normal variant",
                    "properties": {
                        "Normal": {
                            "description": "A very simple enum with unit variants",
                            "oneOf": [
                                {
                                    "enum": [
                                        "C",
                                        "D"
                                    ],
                                    "type": "string"
                                },
                                {
                                    "description": "First variant",
                                    "enum": [
                                        "A"
                                    ],
                                    "type": "string"
                                },
                                {
                                    "description": "Second variant",
                                    "enum": [
                                        "B"
                                    ],
                                    "type": "string"
                                }
                            ]
                        }
                    },
                    "required": [
                        "Normal"
                    ],
                    "type": "object"
                },
                {
                    "additionalProperties": false,
                    "description": "Documentation on the Hardcore variant",
                    "properties": {
                        "Hardcore": {
                            "properties": {
                                "core": {
                                    "description": "A very simple enum with unit variants",
                                    "oneOf": [
                                        {
                                            "enum": [
                                                "C",
                                                "D"
                                            ],
                                            "type": "string"
                                        },
                                        {
                                            "description": "First variant",
                                            "enum": [
                                                "A"
                                            ],
                                            "type": "string"
                                        },
                                        {
                                            "description": "Second variant",
                                            "enum": [
                                                "B"
                                            ],
                                            "type": "string"
                                        }
                                    ]
                                },
                                "hard": {
                                    "type": "string"
                                }
                            },
                            "required": [
                                "hard",
                                "core"
                            ],
                            "type": "object"
                        }
                    },
                    "required": [
                        "Hardcore"
                    ],
                    "type": "object"
                }
            ]
        });

        let expected_converted_schema_object_value = serde_json::json!(
            {
                "description": "A complex tagged enum with unit and struct variants",
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
                        "oneOf": [
                          {
                            "enum": [
                              "C",
                              "D"
                            ],
                            "type": "string"
                          },
                          {
                            "description": "First variant",
                            "enum": [
                              "A"
                            ],
                            "type": "string"
                          },
                          {
                            "description": "Second variant",
                            "enum": [
                              "B"
                            ],
                            "type": "string"
                          }
                        ]
                      },
                      "hard": {
                        "type": "string"
                      }
                    },
                    "required": [
                      "core",
                      "hard"
                    ],
                    "type": "object"
                  },
                  "Normal": {
                    "description": "Override documentation on the Normal variant",
                    "oneOf": [
                      {
                        "enum": [
                          "C",
                          "D"
                        ],
                        "type": "string"
                      },
                      {
                        "description": "First variant",
                        "enum": [
                          "A"
                        ],
                        "type": "string"
                      },
                      {
                        "description": "Second variant",
                        "enum": [
                          "B"
                        ],
                        "type": "string"
                      }
                    ]
                  }
                },
                "type": "object"
              }
        );

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");
        let expected_converted_schema_object: SchemaObject =
            serde_json::from_value(expected_converted_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);

        assert_json_eq!(actual_converted_schema_object, expected_converted_schema_object);
    }

    #[test]
    fn tagged_enum_with_stuct_and_tuple_variants_after_one_of_hoisting() {
        let original_schema_object_value = serde_json::json!({
            "description": "A complex tagged enum with unit and struct variants",
            "oneOf": [
                {
                    "additionalProperties": false,
                    "description": "Override documentation on the Normal variant",
                    "properties": {
                        "Normal": {
                            "description": "A very simple enum with unit variants",
                            "type": "string",
                            "enum": [
                                "C",
                                "D",
                                "A",
                                "B"
                            ]
                        }
                    },
                    "required": [
                        "Normal"
                    ],
                    "type": "object"
                },
                {
                    "additionalProperties": false,
                    "description": "Documentation on the Hardcore variant",
                    "properties": {
                        "Hardcore": {
                            "properties": {
                                "core": {
                                    "description": "A very simple enum with unit variants",
                                    "type": "string",
                                    "enum": [
                                        "C",
                                        "D",
                                        "A",
                                        "B"
                                    ]
                                },
                                "hard": {
                                    "type": "string"
                                }
                            },
                            "required": [
                                "hard",
                                "core"
                            ],
                            "type": "object"
                        }
                    },
                    "required": [
                        "Hardcore"
                    ],
                    "type": "object"
                }
            ]
        });

        let expected_converted_schema_object_value = serde_json::json!(
            {
                "description": "A complex tagged enum with unit and struct variants",
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
                        "type": "string",
                        "enum": [
                            "C",
                            "D",
                            "A",
                            "B"
                        ]
                      },
                      "hard": {
                        "type": "string"
                      }
                    },
                    "required": [
                      "core",
                      "hard"
                    ],
                    "type": "object"
                  },
                  "Normal": {
                    "description": "Override documentation on the Normal variant",
                    "type": "string",
                    "enum": [
                        "C",
                        "D",
                        "A",
                        "B"
                    ]
                  }
                },
                "type": "object"
              }
        );

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");
        let expected_converted_schema_object: SchemaObject =
            serde_json::from_value(expected_converted_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);

        assert_json_eq!(actual_converted_schema_object, expected_converted_schema_object);
    }

    #[test]
    fn untagged_enum_with_empty_variant_before_one_of_hoisting() {
        let original_schema_object_value = serde_json::json!({
            "description": "An untagged enum with a nested enum inside",
            "anyOf": [
                {
                    "description": "Used in case the `one` field is present",
                    "type": "object",
                    "required": [
                        "one"
                    ],
                    "properties": {
                        "one": {
                            "type": "string"
                        }
                    }
                },
                {
                    "description": "Used in case the `two` field is present",
                    "type": "object",
                    "required": [
                        "two"
                    ],
                    "properties": {
                        "two": {
                            "description": "A very simple enum with unit variants",
                            "oneOf": [
                                {
                                    "type": "string",
                                    "enum": [
                                        "C",
                                        "D"
                                    ]
                                },
                                {
                                    "description": "First variant doc-comment",
                                    "type": "string",
                                    "enum": [
                                        "A"
                                    ]
                                },
                                {
                                    "description": "Second variant doc-comment",
                                    "type": "string",
                                    "enum": [
                                        "B"
                                    ]
                                }
                            ]
                        }
                    }
                },
                {
                    "description": "Used in case no fields are present",
                    "type": "object"
                }
            ]
        });

        let expected_converted_schema_object_value = serde_json::json!({
            "description": "An untagged enum with a nested enum inside",
            "type": "object",
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
                {}
            ],
            "properties": {
                "one": {
                    "type": "string"
                },
                "two": {
                    "description": "A very simple enum with unit variants",
                    "oneOf": [
                        {
                            "type": "string",
                            "enum": [
                                "C",
                                "D"
                            ]
                        },
                        {
                            "description": "First variant doc-comment",
                            "type": "string",
                            "enum": [
                                "A"
                            ]
                        },
                        {
                            "description": "Second variant doc-comment",
                            "type": "string",
                            "enum": [
                                "B"
                            ]
                        }
                    ]
                }
            }
        });

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");
        let expected_converted_schema_object: SchemaObject =
            serde_json::from_value(expected_converted_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);

        assert_json_eq!(actual_converted_schema_object, expected_converted_schema_object);
    }

    #[test]
    fn untagged_enum_with_duplicate_field_of_same_shape() {
        let original_schema_object_value = serde_json::json!({
            "description": "Comment for untagged enum ProductImageSelection",
            "anyOf": [
                {
                    "description": "Comment for struct ProductImageCustom",
                    "properties": {
                        "custom": {
                            "description": "Comment for custom field",
                            "type": "string"
                        },
                        "productVersion": {
                            "description": "Comment for product_version field (same on both structs)",
                            "type": "string"
                    }
                },
                    "required": [
                        "productVersion",
                        "custom"
                    ],
                    "type": "object"
                },
                {
                    "description": "Comment for struct ProductImageVersion",
                    "properties": {
                        "productVersion": {
                            "description": "Comment for product_version field (same on both structs)",
                            "type": "string"
                        },
                        "repo": {
                            "description": "Comment for repo field",
                            "nullable": true,
                            "type": "string"
                    }
                },
                    "required": [
                        "productVersion"
                    ],
                    "type": "object"
                }
            ]
        });

        let expected_converted_schema_object_value = serde_json::json!({
            "description": "Comment for untagged enum ProductImageSelection",
            "type": "object",
            "anyOf": [
                {
                    "required": [
                        "custom",
                        "productVersion"
                    ]
                },
                {
                    "required": [
                        "productVersion"
                    ]
                }
            ],
            "properties": {
                "custom": {
                    "description": "Comment for custom field",
                    "type": "string"
                },
                "productVersion": {
                    "description": "Comment for product_version field (same on both structs)",
                    "type": "string"
                        },
                "repo": {
                    "description": "Comment for repo field",
                    "nullable": true,
                    "type": "string"
                }
            }

        });

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");
        let expected_converted_schema_object: SchemaObject =
            serde_json::from_value(expected_converted_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);

        assert_json_eq!(actual_converted_schema_object, expected_converted_schema_object);
    }

    #[test]
    #[should_panic(expected = "Properties for \"two\" are defined multiple times with different shapes")]
    fn invalid_untagged_enum_with_conflicting_variant_fields_before_one_of_hosting() {
        let original_schema_object_value = serde_json::json!({
            "description": "An untagged enum with a nested enum inside",
            "anyOf": [
                {
                    "description": "Used in case the `one` field is present",
                    "type": "object",
                    "required": [
                        "one",
                        "two"
                    ],
                    "properties": {
                        "one": {
                            "type": "string"
                        },
                        "two": {
                            "type": "integer"
                        }
                    }
                },
                {
                    "description": "Used in case the `two` field is present",
                    "type": "object",
                    "required": [
                        "two"
                    ],
                    "properties": {
                        "two": {
                            "description": "A very simple enum with unit variants",
                            "oneOf": [
                                {
                                    "type": "string",
                                    "enum": [
                                        "C",
                                        "D"
                                    ]
                                },
                                {
                                    "description": "First variant doc-comment",
                                    "type": "string",
                                    "enum": [
                                        "A"
                                    ]
                                },
                                {
                                    "description": "Second variant doc-comment",
                                    "type": "string",
                                    "enum": [
                                        "B"
                                    ]
                                }
                            ]
                        }
                    }
                },
                {
                    "description": "Used in case no fields are present",
                    "type": "object"
                }
            ]
        });


        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);
    }

    #[test]
    #[should_panic(expected = "Properties for \"two\" are defined multiple times with different shapes")]
    fn invalid_untagged_enum_with_conflicting_variant_fields_after_one_of_hosting() {
        // NOTE: the oneOf for the second variant has already been hoisted
        let original_schema_object_value = serde_json::json!({
            "description": "An untagged enum with a nested enum inside",
            "anyOf": [
                {
                    "description": "Used in case the `one` field is present",
                    "type": "object",
                    "required": [
                        "one",
                        "two",
                    ],
                    "properties": {
                        "one": {
                            "type": "string"
                        },
                        "two": {
                            "type": "string"
                        }
                    }
                },
                {
                    "description": "Used in case the `two` field is present",
                    "type": "object",
                    "required": [
                        "two"
                    ],
                    "properties": {
                        "two": {
                            "description": "A very simple enum with unit variants",
                            "type": "string",
                            "enum": [
                                "C",
                                "D",
                                "A",
                                "B"
                            ]
                        }
                    }
                },
                {
                    "description": "Used in case no fields are present",
                    "type": "object"
                }
            ]
        });

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        hoist_properties_for_any_of_subschemas(&mut actual_converted_schema_object);
    }
}
