use std::ops::DerefMut;

use schemars::transform::Transform;

use crate::schema::{Schema, SchemaObject, SubschemaValidation, NULL_SCHEMA};

/// Replace the schema with the anyOf subschema and set to nullable when the
/// only other subschema is the nullable entry.
///
/// Used for correcting the schema for optional tagged unit enums.
/// The non-null subschema is hoisted, and nullable will be set to true.
///
/// This will return early without modifications unless:
/// - There are exactly 2 `anyOf` subschemas.
/// - One subschema represents the nullability (ie: it has an `enum` with a single
///   `null` entry, and `nullable` set to true).
///
/// NOTE: This should work regardless of whether other hoisting has been performed or not.
#[derive(Debug, Clone)]
pub struct AnyOfSchemaRewriter;

impl Transform for AnyOfSchemaRewriter {
    fn transform(&mut self, transform_schema: &mut schemars::Schema) {
        // Apply this (self) transform to all subschemas
        schemars::transform::transform_subschemas(self, transform_schema);

        let Some(mut schema) = serde_json::from_value(transform_schema.clone().to_value()).ok() else {
            return;
        };

        hoist_any_of_subschema_with_a_nullable_variant(&mut schema);

        // Convert back to schemars::Schema
        if let Ok(schema) = serde_json::to_value(schema) {
            if let Ok(transformed) = serde_json::from_value(schema) {
                *transform_schema = transformed;
            }
        }
    }
}

fn hoist_any_of_subschema_with_a_nullable_variant(kube_schema: &mut SchemaObject) {
    // Run some initial checks in case there is nothing to do
    let SchemaObject {
        subschemas: Some(subschemas),
        ..
    } = kube_schema
    else {
        return;
    };

    let SubschemaValidation {
        any_of: Some(any_of),
        one_of: None,
    } = subschemas.deref_mut()
    else {
        return;
    };

    if any_of.len() != 2 {
        return;
    }

    let entry_is_null: [bool; 2] = any_of
        .iter()
        .map(|schema| serde_json::to_value(schema).expect("schema should be able to convert to JSON"))
        .map(|value| value == *NULL_SCHEMA)
        .collect::<Vec<_>>()
        .try_into()
        .expect("there should be exactly 2 elements. We checked earlier");

    // Get the `any_of` subschema that isn't the null entry
    let subschema_to_hoist = match entry_is_null {
        [true, false] => &any_of[1],
        [false, true] => &any_of[0],
        _ => return,
    };

    // At this point, we can be reasonably sure we need to hoist the non-null
    // anyOf subschema up to the schema level, and unset the anyOf field.
    // From here, anything that looks wrong will panic instead of return.
    // TODO (@NickLarsenNZ): Return errors instead of panicking, leave panicking up to the
    // infallible schemars::Transform
    let Schema::Object(to_hoist) = subschema_to_hoist else {
        panic!("the non-null anyOf subschema is a bool. That is not expected here");
    };

    let mut to_hoist = to_hoist.clone();

    // Move the metadata into the subschema before hoisting (unless it already has metadata set)
    to_hoist.metadata = to_hoist.metadata.or_else(|| kube_schema.metadata.take());

    // Replace the schema with the non-null subschema
    *kube_schema = to_hoist;

    // Set the schema to nullable (as we know we matched the null variant earlier)
    kube_schema.extensions.insert("nullable".to_owned(), true.into());
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;

    use super::*;

    #[test]
    fn optional_tagged_enum_with_unit_variants() {
        let original_value = serde_json::json!({
            "anyOf": [
                {
                    "description": "A very simple enum with empty variants",
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
                },
                {
                    "enum": [
                        null
                    ],
                    "nullable": true
                }
            ]
        });

        let expected_converted_value = serde_json::json!({
            "description": "A very simple enum with empty variants",
            "nullable": true,
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
        });


        let original: SchemaObject = serde_json::from_value(original_value).expect("valid JSON");
        let expected_converted: SchemaObject =
            serde_json::from_value(expected_converted_value).expect("valid JSON");

        let mut actual_converted = original.clone();
        hoist_any_of_subschema_with_a_nullable_variant(&mut actual_converted);

        assert_json_eq!(actual_converted, expected_converted);
    }

    #[test]
    fn optional_tagged_enum_with_unit_variants_but_also_an_existing_description() {
        let original_value = serde_json::json!({
            "description": "This comment will be lost",
            "anyOf": [
                {
                    "description": "A very simple enum with empty variants",
                    "type": "string",
                    "enum": [
                        "C",
                        "D",
                        "A",
                        "B"
                    ]
                },
                {
                    "enum": [
                        null
                    ],
                    "nullable": true
                }
            ]
        });

        let expected_converted_value = serde_json::json!({
            "description": "A very simple enum with empty variants",
            "nullable": true,
            "type": "string",
            "enum": [
                "C",
                "D",
                "A",
                "B"
            ]
        });


        let original: SchemaObject = serde_json::from_value(original_value).expect("valid JSON");
        let expected_converted: SchemaObject =
            serde_json::from_value(expected_converted_value).expect("valid JSON");

        let mut actual_converted = original.clone();
        hoist_any_of_subschema_with_a_nullable_variant(&mut actual_converted);

        assert_json_eq!(actual_converted, expected_converted);
    }
}
