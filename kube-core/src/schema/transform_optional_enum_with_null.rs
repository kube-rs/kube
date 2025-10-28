use serde_json::Value;

use super::SchemaObject;

/// Drop trailing null on optional enums.
///
/// The nullability is already indicated when "nullable" is set to true.
///
/// NOTE: The trailing null is removed because it's not needed by Kubernetes
/// and makes the CRD more compact by removing redundant information.
pub(crate) fn remove_optional_enum_null_variant(kube_schema: &mut SchemaObject) {
    let SchemaObject {
        enum_values: Some(enum_values),
        extensions,
        ..
    } = kube_schema
    else {
        return;
    };

    // For added safety, check nullability. It should always be true when there is a null variant.
    if let Some(Value::Bool(true)) = extensions.get("nullable") {
        // Don't drop the null entry if it is the only thing in the enum.
        // This is because other hoisting code depends on `kube::core::NULL_SCHEMA` to detect null
        // variants.
        if enum_values.len() > 1 {
            enum_values.retain(|enum_value| enum_value != &Value::Null);
        }
    }
}

#[cfg(test)]
mod tests {
    use assert_json_diff::assert_json_eq;

    use super::*;

    #[test]
    fn optional_enum_with_null() {
        let original_schema_object_value = serde_json::json!({
            "description": "A very simple enum with unit variants without descriptions",
            "enum": [
                "A",
                "B",
                "C",
                "D",
                null
            ],
            "nullable": true
        });

        let expected_converted_schema_object_value = serde_json::json!({
            "description": "A very simple enum with unit variants without descriptions",
            "enum": [
                "A",
                "B",
                "C",
                "D"
            ],
            "nullable": true
        });

        let original_schema_object: SchemaObject =
            serde_json::from_value(original_schema_object_value).expect("valid JSON");
        let expected_converted_schema_object: SchemaObject =
            serde_json::from_value(expected_converted_schema_object_value).expect("valid JSON");

        let mut actual_converted_schema_object = original_schema_object.clone();
        remove_optional_enum_null_variant(&mut actual_converted_schema_object);

        assert_json_eq!(actual_converted_schema_object, expected_converted_schema_object);
    }
}
