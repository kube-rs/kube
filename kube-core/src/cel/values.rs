//! Conversion from `serde_json::Value` to `cel::Value`.
//!
//! This module provides [`json_to_cel`] which recursively converts a JSON value
//! into the CEL value representation used by the `cel` crate. The converted
//! values can then be bound as variables (e.g. `self`, `oldSelf`) in a CEL
//! evaluation context.
//!
//! For schema-aware conversion that respects `format: "date-time"` and
//! `format: "duration"`, use [`json_to_cel_with_schema`] or
//! [`json_to_cel_with_compiled`].

use std::{collections::HashMap, sync::Arc};

use cel::{
    Value,
    objects::{Key, Map},
};

use super::{compilation::CompiledSchema, escaping::escape_field_name};

/// The `format` hint from an OpenAPI schema property.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SchemaFormat {
    /// `format: "date-time"` — strings should be parsed as CEL `Timestamp`.
    DateTime,
    /// `format: "duration"` — strings should be parsed as CEL `Duration`.
    Duration,
    /// No recognized format or not a string type.
    #[default]
    None,
}

impl SchemaFormat {
    /// Extract a `SchemaFormat` from a raw JSON schema node.
    pub(crate) fn from_schema(schema: &serde_json::Value) -> Self {
        match schema.get("format").and_then(|f| f.as_str()) {
            Some("date-time") => SchemaFormat::DateTime,
            Some("duration") => SchemaFormat::Duration,
            _ => SchemaFormat::None,
        }
    }
}

/// Convert a [`serde_json::Value`] into a [`cel::Value`].
///
/// Object keys are escaped via [`escape_field_name`]
/// to handle CEL reserved words and special characters (`.`, `-`, `/`, `_`).
///
/// # Number conversion priority
///
/// JSON numbers are converted using the following priority:
/// 1. `i64` — if the number fits in a signed 64-bit integer
/// 2. `u64` — if the number fits in an unsigned 64-bit integer (but not `i64`)
/// 3. `f64` — for all other numeric values (floating-point)
#[must_use]
pub fn json_to_cel(value: &serde_json::Value) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => convert_number(n),
        serde_json::Value::String(s) => Value::String(Arc::new(s.clone())),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr.iter().map(json_to_cel).collect();
            Value::List(Arc::new(items))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::with_capacity(obj.len());
            for (k, v) in obj {
                map.insert(Key::String(Arc::new(escape_field_name(k))), json_to_cel(v));
            }
            Value::Map(Map { map: Arc::new(map) })
        }
    }
}

fn convert_number(n: &serde_json::Number) -> Value {
    if let Some(i) = n.as_i64() {
        Value::Int(i)
    } else if let Some(u) = n.as_u64() {
        Value::UInt(u)
    } else {
        Value::Float(n.as_f64().unwrap())
    }
}

/// Convert a JSON value to a CEL value, using the raw JSON schema to recognize
/// `format: "date-time"` and `format: "duration"` string fields.
///
/// This recursively walks both the value and the schema in parallel. For string
/// values whose schema specifies a recognized format, the string is parsed into
/// the corresponding CEL type (`Timestamp` or `Duration`). On parse failure,
/// the value falls back to `Value::String`.
#[must_use]
pub fn json_to_cel_with_schema(value: &serde_json::Value, schema: &serde_json::Value) -> Value {
    let format = SchemaFormat::from_schema(schema);
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => convert_number(n),
        serde_json::Value::String(s) => convert_string_with_format(s, &format),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr
                .iter()
                .map(|item| match schema.get("items") {
                    Some(items_schema) => json_to_cel_with_schema(item, items_schema),
                    None => json_to_cel(item),
                })
                .collect();
            Value::List(Arc::new(items))
        }
        serde_json::Value::Object(obj) => {
            let props = schema.get("properties").and_then(|p| p.as_object());
            let additional = schema.get("additionalProperties").filter(|a| a.is_object());

            let mut map = HashMap::with_capacity(obj.len());
            for (k, v) in obj {
                let child_val = if let Some(prop_schema) = props.and_then(|p| p.get(k)) {
                    json_to_cel_with_schema(v, prop_schema)
                } else if let Some(additional_schema) = additional {
                    json_to_cel_with_schema(v, additional_schema)
                } else {
                    json_to_cel(v)
                };
                map.insert(Key::String(Arc::new(escape_field_name(k))), child_val);
            }
            Value::Map(Map { map: Arc::new(map) })
        }
    }
}

/// Convert a JSON value to a CEL value using a pre-compiled [`CompiledSchema`].
///
/// Behaves like [`json_to_cel_with_schema`] but uses the format metadata stored
/// in the compiled schema tree instead of parsing the raw JSON schema.
#[must_use]
pub fn json_to_cel_with_compiled(value: &serde_json::Value, compiled: &CompiledSchema) -> Value {
    match value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => convert_number(n),
        serde_json::Value::String(s) => convert_string_with_format(s, &compiled.format),
        serde_json::Value::Array(arr) => {
            let items: Vec<Value> = arr
                .iter()
                .map(|item| match &compiled.items {
                    Some(items_compiled) => json_to_cel_with_compiled(item, items_compiled),
                    None => json_to_cel(item),
                })
                .collect();
            Value::List(Arc::new(items))
        }
        serde_json::Value::Object(obj) => {
            let mut map = HashMap::with_capacity(obj.len());
            for (k, v) in obj {
                let child_val = if let Some(prop_compiled) = compiled.properties.get(k) {
                    json_to_cel_with_compiled(v, prop_compiled)
                } else if let Some(ref additional) = compiled.additional_properties {
                    json_to_cel_with_compiled(v, additional)
                } else {
                    json_to_cel(v)
                };
                map.insert(Key::String(Arc::new(escape_field_name(k))), child_val);
            }
            Value::Map(Map { map: Arc::new(map) })
        }
    }
}

/// Convert a string using the schema format hint.
fn convert_string_with_format(s: &str, format: &SchemaFormat) -> Value {
    match format {
        SchemaFormat::DateTime => {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return Value::Timestamp(dt);
            }
            Value::String(Arc::new(s.to_string()))
        }
        SchemaFormat::Duration => {
            if let Some(d) = parse_go_duration(s) {
                return Value::Duration(d);
            }
            Value::String(Arc::new(s.to_string()))
        }
        SchemaFormat::None => Value::String(Arc::new(s.to_string())),
    }
}

/// Parse a Go-style duration string into a [`chrono::Duration`].
///
/// Supported units: `h` (hours), `m` (minutes), `s` (seconds), `ms` (milliseconds),
/// `us` (microseconds), `ns` (nanoseconds). Multiple units can be combined
/// (e.g., `"1h30m"`). An optional leading `-` makes the duration negative.
/// The bare string `"0"` is treated as zero duration.
///
/// Returns `None` if the string cannot be parsed.
pub(crate) fn parse_go_duration(input: &str) -> Option<chrono::Duration> {
    let (input, negative) = if let Some(rest) = input.strip_prefix('-') {
        (rest, true)
    } else {
        (input, false)
    };

    if input == "0" {
        return Some(chrono::Duration::zero());
    }

    let mut remaining = input;
    let mut total_nanos: i64 = 0;
    let mut parsed_any = false;

    while !remaining.is_empty() {
        // Parse the numeric part (integer or float)
        let num_end = remaining
            .find(|c: char| !c.is_ascii_digit() && c != '.')
            .unwrap_or(0);
        if num_end == 0 {
            return None; // no digits found
        }
        let num_str = &remaining[..num_end];
        let num: f64 = num_str.parse().ok()?;
        remaining = &remaining[num_end..];

        // Parse the unit suffix
        let (unit_nanos, unit_len) = if remaining.starts_with("ms") {
            (1_000_000i64, 2)
        } else if remaining.starts_with("us") {
            (1_000i64, 2)
        } else if remaining.starts_with("ns") {
            (1i64, 2)
        } else if remaining.starts_with('h') {
            (3_600_000_000_000i64, 1)
        } else if remaining.starts_with('m') {
            (60_000_000_000i64, 1)
        } else if remaining.starts_with('s') {
            (1_000_000_000i64, 1)
        } else {
            return None; // unknown unit
        };

        remaining = &remaining[unit_len..];
        total_nanos += (num * unit_nanos as f64).trunc() as i64;
        parsed_any = true;
    }

    if !parsed_any {
        return None;
    }

    if negative {
        total_nanos = -total_nanos;
    }
    Some(chrono::Duration::nanoseconds(total_nanos))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_null() {
        assert_eq!(json_to_cel(&json!(null)), Value::Null);
    }

    #[test]
    fn test_bool() {
        assert_eq!(json_to_cel(&json!(true)), Value::Bool(true));
        assert_eq!(json_to_cel(&json!(false)), Value::Bool(false));
    }

    #[test]
    fn test_i64() {
        assert_eq!(json_to_cel(&json!(42)), Value::Int(42));
        assert_eq!(json_to_cel(&json!(-1)), Value::Int(-1));
        assert_eq!(json_to_cel(&json!(0)), Value::Int(0));
    }

    #[test]
    fn test_u64_beyond_i64() {
        let big: u64 = (i64::MAX as u64) + 1;
        let v = json_to_cel(&serde_json::Value::Number(serde_json::Number::from(big)));
        assert_eq!(v, Value::UInt(big));
    }

    #[test]
    fn test_float() {
        assert_eq!(json_to_cel(&json!(3.14)), Value::Float(3.14));
        assert_eq!(json_to_cel(&json!(0.0)), Value::Float(0.0));
    }

    #[test]
    fn test_string() {
        assert_eq!(
            json_to_cel(&json!("hello")),
            Value::String(Arc::new("hello".into()))
        );
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(json_to_cel(&json!("")), Value::String(Arc::new(String::new())));
    }

    #[test]
    fn test_array_mixed() {
        let v = json_to_cel(&json!([1, "two", true, null]));
        let expected = Value::List(Arc::new(vec![
            Value::Int(1),
            Value::String(Arc::new("two".into())),
            Value::Bool(true),
            Value::Null,
        ]));
        assert_eq!(v, expected);
    }

    #[test]
    fn test_empty_array() {
        assert_eq!(json_to_cel(&json!([])), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_object() {
        let v = json_to_cel(&json!({"name": "test", "count": 5}));
        if let Value::Map(map) = v {
            assert_eq!(
                map.map.get(&Key::String(Arc::new("name".into()))),
                Some(&Value::String(Arc::new("test".into())))
            );
            assert_eq!(
                map.map.get(&Key::String(Arc::new("count".into()))),
                Some(&Value::Int(5))
            );
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_empty_object() {
        let v = json_to_cel(&json!({}));
        if let Value::Map(map) = v {
            assert!(map.map.is_empty());
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn test_nested_structure() {
        let v = json_to_cel(&json!({
            "spec": {
                "replicas": 3,
                "items": [1, 2, 3]
            }
        }));
        if let Value::Map(outer) = v {
            let spec = outer.map.get(&Key::String(Arc::new("spec".into()))).unwrap();
            if let Value::Map(inner) = spec {
                assert_eq!(
                    inner.map.get(&Key::String(Arc::new("replicas".into()))),
                    Some(&Value::Int(3))
                );
                assert_eq!(
                    inner.map.get(&Key::String(Arc::new("items".into()))),
                    Some(&Value::List(Arc::new(vec![
                        Value::Int(1),
                        Value::Int(2),
                        Value::Int(3),
                    ])))
                );
            } else {
                panic!("expected inner Map");
            }
        } else {
            panic!("expected outer Map");
        }
    }

    #[test]
    fn test_number_priority() {
        // i64 range → Int
        assert_eq!(json_to_cel(&json!(42)), Value::Int(42));
        // u64 beyond i64 → UInt
        let big: u64 = (i64::MAX as u64) + 1;
        assert_eq!(
            json_to_cel(&serde_json::Value::Number(serde_json::Number::from(big))),
            Value::UInt(big)
        );
        // float → Float
        assert_eq!(json_to_cel(&json!(1.5)), Value::Float(1.5));
    }

    // ── parse_go_duration tests ─────────────────────────────────────

    #[test]
    fn parse_duration_hours() {
        assert_eq!(parse_go_duration("1h"), Some(chrono::Duration::hours(1)));
    }

    #[test]
    fn parse_duration_minutes() {
        assert_eq!(parse_go_duration("30m"), Some(chrono::Duration::minutes(30)));
    }

    #[test]
    fn parse_duration_seconds() {
        assert_eq!(parse_go_duration("45s"), Some(chrono::Duration::seconds(45)));
    }

    #[test]
    fn parse_duration_milliseconds() {
        assert_eq!(
            parse_go_duration("500ms"),
            Some(chrono::Duration::milliseconds(500))
        );
    }

    #[test]
    fn parse_duration_microseconds() {
        assert_eq!(
            parse_go_duration("100us"),
            Some(chrono::Duration::microseconds(100))
        );
    }

    #[test]
    fn parse_duration_nanoseconds() {
        assert_eq!(
            parse_go_duration("999ns"),
            Some(chrono::Duration::nanoseconds(999))
        );
    }

    #[test]
    fn parse_duration_compound() {
        assert_eq!(
            parse_go_duration("1h30m"),
            Some(chrono::Duration::hours(1) + chrono::Duration::minutes(30))
        );
        assert_eq!(
            parse_go_duration("1h30m10s"),
            Some(chrono::Duration::hours(1) + chrono::Duration::minutes(30) + chrono::Duration::seconds(10))
        );
    }

    #[test]
    fn parse_duration_negative() {
        assert_eq!(parse_go_duration("-1h"), Some(chrono::Duration::hours(-1)));
        assert_eq!(parse_go_duration("-30s"), Some(chrono::Duration::seconds(-30)));
    }

    #[test]
    fn parse_duration_zero() {
        assert_eq!(parse_go_duration("0"), Some(chrono::Duration::zero()));
    }

    #[test]
    fn parse_duration_invalid() {
        assert_eq!(parse_go_duration(""), None);
        assert_eq!(parse_go_duration("abc"), None);
        assert_eq!(parse_go_duration("1x"), None);
        assert_eq!(parse_go_duration("h"), None);
    }

    // ── Schema-aware conversion tests ───────────────────────────────

    #[test]
    fn timestamp_parsed_from_schema() {
        let schema = json!({
            "type": "string",
            "format": "date-time"
        });
        let value = json!("2024-01-01T00:00:00Z");
        let result = json_to_cel_with_schema(&value, &schema);
        assert!(matches!(result, Value::Timestamp(_)));
    }

    #[test]
    fn timestamp_parse_failure_falls_back_to_string() {
        let schema = json!({
            "type": "string",
            "format": "date-time"
        });
        let value = json!("not-a-date");
        let result = json_to_cel_with_schema(&value, &schema);
        assert_eq!(result, Value::String(Arc::new("not-a-date".into())));
    }

    #[test]
    fn duration_parsed_from_schema() {
        let schema = json!({
            "type": "string",
            "format": "duration"
        });
        let value = json!("1h30m");
        let result = json_to_cel_with_schema(&value, &schema);
        assert!(matches!(result, Value::Duration(_)));
    }

    #[test]
    fn duration_parse_failure_falls_back_to_string() {
        let schema = json!({
            "type": "string",
            "format": "duration"
        });
        let value = json!("not-a-duration");
        let result = json_to_cel_with_schema(&value, &schema);
        assert_eq!(result, Value::String(Arc::new("not-a-duration".into())));
    }

    #[test]
    fn nested_object_properties_format() {
        let schema = json!({
            "type": "object",
            "properties": {
                "createdAt": {
                    "type": "string",
                    "format": "date-time"
                },
                "name": {
                    "type": "string"
                },
                "timeout": {
                    "type": "string",
                    "format": "duration"
                }
            }
        });
        let value = json!({
            "createdAt": "2024-06-15T10:30:00Z",
            "name": "test",
            "timeout": "30s"
        });
        let result = json_to_cel_with_schema(&value, &schema);
        if let Value::Map(map) = result {
            assert!(matches!(
                map.map.get(&Key::String(Arc::new("createdAt".into()))),
                Some(Value::Timestamp(_))
            ));
            assert!(matches!(
                map.map.get(&Key::String(Arc::new("name".into()))),
                Some(Value::String(_))
            ));
            assert!(matches!(
                map.map.get(&Key::String(Arc::new("timeout".into()))),
                Some(Value::Duration(_))
            ));
        } else {
            panic!("expected Map");
        }
    }

    #[test]
    fn array_items_format() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "string",
                "format": "date-time"
            }
        });
        let value = json!(["2024-01-01T00:00:00Z", "2024-06-15T12:00:00+09:00"]);
        let result = json_to_cel_with_schema(&value, &schema);
        if let Value::List(items) = result {
            assert!(matches!(items[0], Value::Timestamp(_)));
            assert!(matches!(items[1], Value::Timestamp(_)));
        } else {
            panic!("expected List");
        }
    }

    #[test]
    fn no_format_leaves_string_unchanged() {
        let schema = json!({
            "type": "string"
        });
        let value = json!("2024-01-01T00:00:00Z");
        let result = json_to_cel_with_schema(&value, &schema);
        assert_eq!(result, Value::String(Arc::new("2024-01-01T00:00:00Z".into())));
    }

    #[test]
    fn json_to_cel_unchanged_with_no_schema() {
        // Original json_to_cel should still work as before
        let value = json!("2024-01-01T00:00:00Z");
        let result = json_to_cel(&value);
        assert_eq!(result, Value::String(Arc::new("2024-01-01T00:00:00Z".into())));
    }
}
