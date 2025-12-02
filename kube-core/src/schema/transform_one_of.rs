use std::ops::DerefMut;

use schemars::transform::Transform;

use crate::schema::{Schema, SchemaObject, SubschemaValidation};

/// Merge oneOf subschema enums and consts into a schema level enum.
///
/// Used for correcting the schema for tagged enums with unit variants.
///
/// NOTE: Subschema descriptions are lost when they are combined into a single
/// enum of the same type.
///
/// This will return early without modifications unless:
/// - There are `oneOf` subschemas (not empty).
/// - Each subschema contains an `enum` or `const`.
///
/// Subschemas must define a type, and they must be the same for all.
///
/// NOTE: This should work regardless of whether other hoisting has been
/// performed or not.
#[derive(Debug, Clone)]
pub struct OneOfSchemaRewriter;

impl Transform for OneOfSchemaRewriter {
    fn transform(&mut self, transform_schema: &mut schemars::Schema) {
        // Apply this (self) transform to all subschemas
        schemars::transform::transform_subschemas(self, transform_schema);

        let Some(mut schema) = serde_json::from_value(transform_schema.clone().to_value()).ok() else {
            return;
        };

        hoist_one_of_enum_with_unit_variants(&mut schema);

        // Convert back to schemars::Schema
        if let Ok(schema) = serde_json::to_value(schema) {
            if let Ok(transformed) = serde_json::from_value(schema) {
                *transform_schema = transformed;
            }
        }
    }
}

fn hoist_one_of_enum_with_unit_variants(kube_schema: &mut SchemaObject) {
    // Run some initial checks in case there is nothing to do
    let SchemaObject {
        subschemas: Some(subschemas),
        ..
    } = kube_schema
    else {
        return;
    };

    let SubschemaValidation {
        any_of: None,
        one_of: Some(one_of),
    } = subschemas.deref_mut()
    else {
        return;
    };

    if one_of.is_empty() {
        return;
    }

    // At this point, we can be reasonably sure we need to hoist the oneOf
    // subschema enums and types up to the schema level, and unset the oneOf field.
    // From here, anything that looks wrong will panic instead of return.
    // TODO (@NickLarsenNZ): Return errors instead of panicking, leave panicking up to the
    // infallible schemars::Transform

    // Prepare to ensure each variant schema has a type
    let mut types = one_of.iter().map(|schema| match schema {
        Schema::Object(SchemaObject {
            instance_type: Some(r#type),
            ..
        }) => r#type,
        Schema::Object(untyped) => panic!("oneOf variants need to define a type: {untyped:#?}"),
        Schema::Bool(_) => panic!("oneOf variants can not be of type boolean"),
    });

    // Get the first type
    let variant_type = types.next().expect("at this point, there must be a type");
    // Ensure all variant types match it
    if types.any(|r#type| r#type != variant_type) {
        panic!("oneOf variants must all have the same type");
    }

    // For each `oneOf` entry, iterate over the `enum` and `const` values.
    // Panic on an entry that doesn't contain an `enum` or `const`.
    let new_enums = one_of
        .iter()
        .flat_map(|schema| match schema {
            Schema::Object(SchemaObject {
                enum_values: Some(r#enum),
                ..
            }) => r#enum.clone(),
            Schema::Object(SchemaObject { other, .. }) => other.get("const").cloned().into_iter().collect(),
            Schema::Bool(_) => panic!("oneOf variants can not be of type boolean"),
        })
        .collect::<Vec<_>>();

    // If there are no enums (or consts converted to enums) in the oneOf subschemas, there is nothing more to do here.
    // For example, when the schema has `properties` and `required`, so we leave that for the properties hoister.
    if new_enums.is_empty() {
        return;
    }

    // Merge the enums (extend just to be safe)
    kube_schema.enum_values.get_or_insert_default().extend(new_enums);

    // Hoist the type
    kube_schema.instance_type = Some(variant_type.clone());

    // Clear the oneOf subschemas
    subschemas.one_of = None;
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;

    use super::*;

    #[test]
    fn tagged_enum_with_unit_variants() {
        let original_value = serde_json::json!({
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
                },
            ]
        });

        let expected_converted_value = serde_json::json!({
            "description": "A very simple enum with unit variants",
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
        hoist_one_of_enum_with_unit_variants(&mut actual_converted);

        assert_json_eq!(actual_converted, expected_converted);
    }
}
