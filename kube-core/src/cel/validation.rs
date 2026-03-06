//! Schema tree walking and CEL rule evaluation for Kubernetes CRD validation.
//!
//! This module provides [`Validator`] which recursively walks an OpenAPI schema,
//! compiles `x-kubernetes-validations` rules, evaluates them against object data,
//! and collects [`ValidationError`]s.

use super::{
    compilation::{CompilationError, CompilationResult, CompiledSchema, compile_schema_validations},
    values::{json_to_cel_with_compiled, json_to_cel_with_schema},
};
use cel::Context;

/// The kind of error that occurred during validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// CEL expression syntax error.
    CompilationFailure,
    /// Malformed rule JSON.
    InvalidRule,
    /// Rule evaluated to `false`.
    ValidationFailure,
    /// Rule returned a non-bool value.
    InvalidResult,
    /// Runtime evaluation error.
    EvaluationError,
}

/// An error produced when a CEL validation rule fails.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationError {
    /// The CEL expression that failed.
    pub rule: String,
    /// Human-readable error message.
    pub message: String,
    /// JSON path to the field (e.g., "spec.replicas").
    pub field_path: String,
    /// Machine-readable reason (e.g., "FieldValueInvalid").
    pub reason: Option<String>,
    /// Classification of the error.
    pub kind: ErrorKind,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.field_path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.field_path, self.message)
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validates Kubernetes objects against CRD schema CEL validation rules.
///
/// Walks the OpenAPI schema tree, compiles `x-kubernetes-validations` rules at
/// each node, and evaluates them against the corresponding object values.
///
/// For repeated validation against the same schema, use [`compile_schema`](super::compilation::compile_schema) +
/// [`validate_compiled`](Validator::validate_compiled) to avoid re-compilation.
///
/// # Thread Safety
///
/// `Validator` is `Send + Sync` and can be shared across threads.
#[derive(Clone, Debug)]
pub struct Validator {
    _private: (),
}

impl Validator {
    /// Create a new `Validator`.
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Validate an object against a CRD schema's CEL validation rules.
    ///
    /// Compiles rules on each call. For repeated validation against the same
    /// schema, prefer [`compile_schema`](super::compilation::compile_schema) + [`validate_compiled`](Self::validate_compiled).
    #[must_use]
    pub fn validate(
        &self,
        schema: &serde_json::Value,
        object: &serde_json::Value,
        old_object: Option<&serde_json::Value>,
    ) -> Vec<ValidationError> {
        let mut base_ctx = Context::default();
        super::register_all(&mut base_ctx);
        let mut errors = Vec::new();
        self.walk_schema(schema, object, old_object, String::new(), &mut errors, &base_ctx);
        errors
    }

    /// Validate an object using a pre-compiled schema tree.
    ///
    /// Use [`compile_schema`](super::compilation::compile_schema) to build the [`CompiledSchema`], then call this
    /// method for each object to validate — rules are compiled only once.
    #[must_use]
    pub fn validate_compiled(
        &self,
        compiled: &CompiledSchema,
        object: &serde_json::Value,
        old_object: Option<&serde_json::Value>,
    ) -> Vec<ValidationError> {
        let mut base_ctx = Context::default();
        super::register_all(&mut base_ctx);
        let mut errors = Vec::new();
        self.walk_compiled(
            compiled,
            object,
            old_object,
            String::new(),
            &mut errors,
            &base_ctx,
        );
        errors
    }

    // ── Schema-based walking (compiles on each call) ────────────────

    fn walk_schema(
        &self,
        schema: &serde_json::Value,
        value: &serde_json::Value,
        old_value: Option<&serde_json::Value>,
        path: String,
        errors: &mut Vec<ValidationError>,
        base_ctx: &Context<'_>,
    ) {
        let cel_value = json_to_cel_with_schema(value, schema);
        let cel_old = old_value.map(|o| json_to_cel_with_schema(o, schema));
        self.evaluate_validations(schema, &cel_value, cel_old.as_ref(), &path, errors, base_ctx);

        if let (Some(properties), Some(obj)) = (
            schema.get("properties").and_then(|p| p.as_object()),
            value.as_object(),
        ) {
            for (prop_name, prop_schema) in properties {
                if let Some(child_value) = obj.get(prop_name) {
                    let child_old = old_value.and_then(|o| o.get(prop_name));
                    let child_path = join_path(&path, prop_name);
                    self.walk_schema(prop_schema, child_value, child_old, child_path, errors, base_ctx);
                }
            }
        }

        if let (Some(items_schema), Some(arr)) = (schema.get("items"), value.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                let old_item = old_value.and_then(|o| o.as_array()).and_then(|a| a.get(i));
                let item_path = join_path_index(&path, i);
                self.walk_schema(items_schema, item, old_item, item_path, errors, base_ctx);
            }
        }

        if let (Some(additional_schema), Some(obj)) = (
            schema.get("additionalProperties").filter(|a| a.is_object()),
            value.as_object(),
        ) {
            let known: std::collections::HashSet<&str> = schema
                .get("properties")
                .and_then(|p| p.as_object())
                .map(|p| p.keys().map(|k| k.as_str()).collect())
                .unwrap_or_default();

            for (key, val) in obj {
                if known.contains(key.as_str()) {
                    continue;
                }
                let old_val = old_value.and_then(|o| o.get(key));
                let child_path = join_path(&path, key);
                self.walk_schema(additional_schema, val, old_val, child_path, errors, base_ctx);
            }
        }
    }

    fn evaluate_validations(
        &self,
        schema: &serde_json::Value,
        cel_value: &cel::Value,
        cel_old: Option<&cel::Value>,
        path: &str,
        errors: &mut Vec<ValidationError>,
        base_ctx: &Context<'_>,
    ) {
        let compiled = compile_schema_validations(schema);
        self.evaluate_compiled_results(&compiled, cel_value, cel_old, path, errors, base_ctx);
    }

    // ── CompiledSchema-based walking ────────────────────────────────

    fn walk_compiled(
        &self,
        compiled: &CompiledSchema,
        value: &serde_json::Value,
        old_value: Option<&serde_json::Value>,
        path: String,
        errors: &mut Vec<ValidationError>,
        base_ctx: &Context<'_>,
    ) {
        let cel_value = json_to_cel_with_compiled(value, compiled);
        let cel_old = old_value.map(|o| json_to_cel_with_compiled(o, compiled));
        self.evaluate_compiled_results(
            &compiled.validations,
            &cel_value,
            cel_old.as_ref(),
            &path,
            errors,
            base_ctx,
        );

        if let Some(obj) = value.as_object() {
            for (prop_name, child_compiled) in &compiled.properties {
                if let Some(child_value) = obj.get(prop_name) {
                    let child_old = old_value.and_then(|o| o.get(prop_name));
                    let child_path = join_path(&path, prop_name);
                    self.walk_compiled(
                        child_compiled,
                        child_value,
                        child_old,
                        child_path,
                        errors,
                        base_ctx,
                    );
                }
            }
        }

        if let (Some(items_compiled), Some(arr)) = (&compiled.items, value.as_array()) {
            for (i, item) in arr.iter().enumerate() {
                let old_item = old_value.and_then(|o| o.as_array()).and_then(|a| a.get(i));
                let item_path = join_path_index(&path, i);
                self.walk_compiled(items_compiled, item, old_item, item_path, errors, base_ctx);
            }
        }

        if let (Some(additional_compiled), Some(obj)) = (&compiled.additional_properties, value.as_object()) {
            for (key, val) in obj {
                if compiled.properties.contains_key(key) {
                    continue;
                }
                let old_val = old_value.and_then(|o| o.get(key));
                let child_path = join_path(&path, key);
                self.walk_compiled(additional_compiled, val, old_val, child_path, errors, base_ctx);
            }
        }
    }

    // ── Shared evaluation logic ─────────────────────────────────────

    fn evaluate_compiled_results(
        &self,
        results: &[Result<CompilationResult, CompilationError>],
        cel_value: &cel::Value,
        cel_old: Option<&cel::Value>,
        path: &str,
        errors: &mut Vec<ValidationError>,
        base_ctx: &Context<'_>,
    ) {
        // Create a node-level scope once with self/oldSelf bound
        let mut node_ctx = base_ctx.new_inner_scope();
        node_ctx.add_variable_from_value("self", cel_value.clone());
        if let Some(old) = cel_old {
            node_ctx.add_variable_from_value("oldSelf", old.clone());
        }

        for result in results {
            match result {
                Ok(cr) => {
                    self.evaluate_rule(cr, &node_ctx, cel_old, path, errors);
                }
                Err(CompilationError::Parse { rule, source }) => {
                    errors.push(ValidationError {
                        rule: rule.clone(),
                        message: format!("failed to compile rule \"{rule}\": {source}"),
                        field_path: path.to_string(),
                        reason: None,
                        kind: ErrorKind::CompilationFailure,
                    });
                }
                Err(CompilationError::InvalidRule(e)) => {
                    errors.push(ValidationError {
                        rule: String::new(),
                        message: format!("invalid rule definition: {e}"),
                        field_path: path.to_string(),
                        reason: None,
                        kind: ErrorKind::InvalidRule,
                    });
                }
            }
        }
    }

    fn evaluate_rule(
        &self,
        cr: &CompilationResult,
        node_ctx: &Context<'_>,
        cel_old: Option<&cel::Value>,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // Handle transition rules
        if cr.is_transition_rule && cel_old.is_none() && cr.rule.optional_old_self != Some(true) {
            return; // skip transition rule without old value
        }

        // optionalOldSelf: true + no old object → child scope with oldSelf = null
        let use_null_old_self = cel_old.is_none() && cr.rule.optional_old_self == Some(true);
        let null_scope;
        let effective_ctx: &Context<'_> = if use_null_old_self {
            null_scope = {
                let mut s = node_ctx.new_inner_scope();
                s.add_variable_from_value("oldSelf", cel::Value::Null);
                s
            };
            &null_scope
        } else {
            node_ctx
        };

        let result = cr.program.execute(effective_ctx);
        let error_path = effective_path(path, cr.rule.field_path.as_deref());

        match result {
            Ok(cel::Value::Bool(true)) => {
                // Validation passed
            }
            Ok(cel::Value::Bool(false)) => {
                let message = self.resolve_message(cr, effective_ctx);
                errors.push(ValidationError {
                    rule: cr.rule.rule.clone(),
                    message,
                    field_path: error_path,
                    reason: cr.rule.reason.clone(),
                    kind: ErrorKind::ValidationFailure,
                });
            }
            Ok(_) => {
                errors.push(ValidationError {
                    rule: cr.rule.rule.clone(),
                    message: format!("rule \"{}\" did not evaluate to bool", cr.rule.rule),
                    field_path: error_path,
                    reason: None,
                    kind: ErrorKind::InvalidResult,
                });
            }
            Err(e) => {
                errors.push(ValidationError {
                    rule: cr.rule.rule.clone(),
                    message: format!("rule evaluation error: {e}"),
                    field_path: error_path,
                    reason: None,
                    kind: ErrorKind::EvaluationError,
                });
            }
        }
    }

    /// Resolve the error message: try messageExpression first, fall back to
    /// static message, then default.
    fn resolve_message(&self, cr: &CompilationResult, ctx: &Context<'_>) -> String {
        if let Some(ref msg_prog) = cr.message_program
            && let Ok(cel::Value::String(s)) = msg_prog.execute(ctx)
        {
            return (*s).clone();
        }
        cr.rule
            .message
            .clone()
            .unwrap_or_else(|| format!("failed rule: {}", cr.rule.rule))
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience function to validate without creating a [`Validator`] instance.
///
/// See [`Validator::validate`] for details.
#[must_use]
pub fn validate(
    schema: &serde_json::Value,
    object: &serde_json::Value,
    old_object: Option<&serde_json::Value>,
) -> Vec<ValidationError> {
    Validator::new().validate(schema, object, old_object)
}

/// Convenience function to validate using a pre-compiled schema.
///
/// See [`Validator::validate_compiled`] for details.
#[must_use]
pub fn validate_compiled(
    compiled: &CompiledSchema,
    object: &serde_json::Value,
    old_object: Option<&serde_json::Value>,
) -> Vec<ValidationError> {
    Validator::new().validate_compiled(compiled, object, old_object)
}

// ── Path helpers ────────────────────────────────────────────────────

fn effective_path(base_path: &str, rule_field_path: Option<&str>) -> String {
    match rule_field_path {
        Some(fp) if fp.starts_with('.') => format!("{base_path}{fp}"),
        Some(fp) if !base_path.is_empty() => format!("{base_path}.{fp}"),
        Some(fp) => fp.to_string(),
        None => base_path.to_string(),
    }
}

fn join_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        segment.to_string()
    } else {
        format!("{base}.{segment}")
    }
}

fn join_path_index(base: &str, index: usize) -> String {
    if base.is_empty() {
        format!("[{index}]")
    } else {
        format!("{base}[{index}]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cel::compilation::compile_schema;
    use serde_json::json;

    fn make_schema(validations: serde_json::Value) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "replicas": {"type": "integer"},
                "name": {"type": "string"}
            },
            "x-kubernetes-validations": validations
        })
    }

    #[test]
    fn validation_passes() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0", "message": "must be non-negative"}
        ]));
        let obj = json!({"replicas": 3, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn validation_fails() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0", "message": "must be non-negative"}
        ]));
        let obj = json!({"replicas": -1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "must be non-negative");
        assert_eq!(errors[0].rule, "self.replicas >= 0");
    }

    #[test]
    fn default_message_when_none() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0"}
        ]));
        let obj = json!({"replicas": -1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("self.replicas >= 0"));
    }

    #[test]
    fn reason_preserved() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0", "message": "bad", "reason": "FieldValueInvalid"}
        ]));
        let obj = json!({"replicas": -1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors[0].reason.as_deref(), Some("FieldValueInvalid"));
    }

    #[test]
    fn transition_rule_skipped_without_old_object() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= oldSelf.replicas", "message": "cannot scale down"}
        ]));
        let obj = json!({"replicas": 1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn transition_rule_evaluated_with_old_object() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= oldSelf.replicas", "message": "cannot scale down"}
        ]));
        let obj = json!({"replicas": 1, "name": "app"});
        let old = json!({"replicas": 3, "name": "app"});
        let errors = validate(&schema, &obj, Some(&old));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "cannot scale down");
    }

    #[test]
    fn transition_rule_passes() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= oldSelf.replicas", "message": "cannot scale down"}
        ]));
        let obj = json!({"replicas": 5, "name": "app"});
        let old = json!({"replicas": 3, "name": "app"});
        let errors = validate(&schema, &obj, Some(&old));
        assert!(errors.is_empty());
    }

    #[test]
    fn nested_property_field_path() {
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "replicas": {
                            "type": "integer",
                            "x-kubernetes-validations": [
                                {"rule": "self >= 0", "message": "must be non-negative"}
                            ]
                        }
                    }
                }
            }
        });
        let obj = json!({"spec": {"replicas": -1}});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "spec.replicas");
        assert_eq!(errors[0].message, "must be non-negative");
    }

    #[test]
    fn array_items_validation() {
        let schema = json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"}
                        },
                        "x-kubernetes-validations": [
                            {"rule": "self.name.size() > 0", "message": "name required"}
                        ]
                    }
                }
            }
        });
        let obj = json!({
            "items": [
                {"name": "good"},
                {"name": ""},
                {"name": "also-good"}
            ]
        });
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "items[1]");
        assert_eq!(errors[0].message, "name required");
    }

    #[test]
    fn missing_field_not_validated() {
        let schema = json!({
            "type": "object",
            "properties": {
                "optional_field": {
                    "type": "integer",
                    "x-kubernetes-validations": [
                        {"rule": "self >= 0", "message": "must be non-negative"}
                    ]
                }
            }
        });
        let obj = json!({});
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn multiple_rules_partial_failure() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0", "message": "non-negative"},
            {"rule": "self.name.size() > 0", "message": "name required"}
        ]));
        let obj = json!({"replicas": -1, "name": ""});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn compilation_error_reported() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >="}
        ]));
        let obj = json!({"replicas": 1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("failed to compile"));
    }

    #[test]
    fn no_validations_no_errors() {
        let schema = json!({
            "type": "object",
            "properties": {
                "replicas": {"type": "integer"}
            }
        });
        let obj = json!({"replicas": -1});
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn display_with_field_path() {
        let err = ValidationError {
            rule: "self >= 0".into(),
            message: "must be non-negative".into(),
            field_path: "spec.replicas".into(),
            reason: None,
            kind: ErrorKind::ValidationFailure,
        };
        assert_eq!(err.to_string(), "spec.replicas: must be non-negative");
    }

    #[test]
    fn display_without_field_path() {
        let err = ValidationError {
            rule: "self >= 0".into(),
            message: "must be non-negative".into(),
            field_path: String::new(),
            reason: None,
            kind: ErrorKind::ValidationFailure,
        };
        assert_eq!(err.to_string(), "must be non-negative");
    }

    #[test]
    fn validator_default() {
        let v = Validator::default();
        let schema = make_schema(json!([{"rule": "self.replicas >= 0"}]));
        let obj = json!({"replicas": 1, "name": "app"});
        assert!(v.validate(&schema, &obj, None).is_empty());
    }

    #[test]
    fn additional_properties_walking() {
        let schema = json!({
            "type": "object",
            "additionalProperties": {
                "type": "integer",
                "x-kubernetes-validations": [
                    {"rule": "self >= 0", "message": "must be non-negative"}
                ]
            }
        });
        let obj = json!({"a": 1, "b": -1, "c": 5});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "b");
    }

    // ── Phase 5 tests ───────────────────────────────────────────────

    #[test]
    fn message_expression_produces_dynamic_message() {
        let schema = make_schema(json!([{
            "rule": "self.replicas >= 0",
            "message": "static fallback",
            "messageExpression": "'replicas is ' + string(self.replicas) + ', must be >= 0'"
        }]));
        let obj = json!({"replicas": -5, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "replicas is -5, must be >= 0");
    }

    #[test]
    fn message_expression_falls_back_to_static() {
        let schema = make_schema(json!([{
            "rule": "self.replicas >= 0",
            "message": "static message",
            "messageExpression": "invalid >="
        }]));
        let obj = json!({"replicas": -1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        // messageExpression failed to compile → falls back to static message
        assert_eq!(errors[0].message, "static message");
    }

    #[test]
    fn optional_old_self_evaluated_on_create() {
        let schema = make_schema(json!([{
            "rule": "oldSelf == null || self.replicas >= oldSelf.replicas",
            "message": "cannot scale down",
            "optionalOldSelf": true
        }]));
        // Create (no old object): rule is evaluated with oldSelf = null
        let obj = json!({"replicas": 1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty()); // oldSelf == null → true
    }

    #[test]
    fn optional_old_self_with_old_object() {
        let schema = make_schema(json!([{
            "rule": "oldSelf == null || self.replicas >= oldSelf.replicas",
            "message": "cannot scale down",
            "optionalOldSelf": true
        }]));
        let obj = json!({"replicas": 1, "name": "app"});
        let old = json!({"replicas": 3, "name": "app"});
        let errors = validate(&schema, &obj, Some(&old));
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].message, "cannot scale down");
    }

    #[test]
    fn optional_old_self_false_still_skips() {
        let schema = make_schema(json!([{
            "rule": "self.replicas >= oldSelf.replicas",
            "message": "cannot scale down",
            "optionalOldSelf": false
        }]));
        let obj = json!({"replicas": 1, "name": "app"});
        // optionalOldSelf: false → transition rule skipped on create
        let errors = validate(&schema, &obj, None);
        assert!(errors.is_empty());
    }

    #[test]
    fn validate_compiled_matches_validate() {
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "x-kubernetes-validations": [
                        {"rule": "self.replicas >= 0", "message": "non-negative"}
                    ],
                    "properties": {
                        "replicas": {"type": "integer"}
                    }
                }
            }
        });
        let obj = json!({"spec": {"replicas": -1}});

        let errors_schema = validate(&schema, &obj, None);
        let compiled = compile_schema(&schema);
        let errors_compiled = validate_compiled(&compiled, &obj, None);

        assert_eq!(errors_schema.len(), errors_compiled.len());
        assert_eq!(errors_schema[0].message, errors_compiled[0].message);
        assert_eq!(errors_schema[0].field_path, errors_compiled[0].field_path);
    }

    #[test]
    fn validate_compiled_reuse() {
        let schema = json!({
            "type": "object",
            "x-kubernetes-validations": [
                {"rule": "self.x > 0", "message": "x must be positive"}
            ],
            "properties": {"x": {"type": "integer"}}
        });
        let compiled = compile_schema(&schema);

        // Validate multiple objects with the same compiled schema
        assert_eq!(validate_compiled(&compiled, &json!({"x": 1}), None).len(), 0);
        assert_eq!(validate_compiled(&compiled, &json!({"x": -1}), None).len(), 1);
        assert_eq!(validate_compiled(&compiled, &json!({"x": 5}), None).len(), 0);
        assert_eq!(validate_compiled(&compiled, &json!({"x": 0}), None).len(), 1);
    }

    // ── fieldPath override tests ────────────────────────────────────

    #[test]
    fn fieldpath_overrides_auto_path() {
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"}
                    },
                    "x-kubernetes-validations": [
                        {"rule": "self.x >= 0", "message": "bad", "fieldPath": ".spec.x"}
                    ]
                }
            }
        });
        let obj = json!({"spec": {"x": -1}});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "spec.spec.x");
    }

    #[test]
    fn fieldpath_without_dot() {
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "x-kubernetes-validations": [
                        {"rule": "self.name.size() > 0", "message": "bad", "fieldPath": "name"}
                    ]
                }
            }
        });
        let obj = json!({"spec": {"name": ""}});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "spec.name");
    }

    #[test]
    fn fieldpath_at_root() {
        let schema = json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer"}
            },
            "x-kubernetes-validations": [
                {"rule": "self.x >= 0", "message": "bad", "fieldPath": ".spec.x"}
            ]
        });
        let obj = json!({"x": -1});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, ".spec.x");
    }

    #[test]
    fn fieldpath_none_uses_auto() {
        let schema = json!({
            "type": "object",
            "properties": {
                "spec": {
                    "type": "object",
                    "properties": {
                        "x": {"type": "integer"}
                    },
                    "x-kubernetes-validations": [
                        {"rule": "self.x >= 0", "message": "bad"}
                    ]
                }
            }
        });
        let obj = json!({"spec": {"x": -1}});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].field_path, "spec");
    }

    // ── ErrorKind tests ─────────────────────────────────────────────

    #[test]
    fn error_kind_compilation_failure() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >="}
        ]));
        let obj = json!({"replicas": 1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, ErrorKind::CompilationFailure);
    }

    #[test]
    fn error_kind_validation_failure() {
        let schema = make_schema(json!([
            {"rule": "self.replicas >= 0", "message": "must be non-negative"}
        ]));
        let obj = json!({"replicas": -1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, ErrorKind::ValidationFailure);
    }

    #[test]
    fn error_kind_evaluation_error() {
        let schema = make_schema(json!([
            {"rule": "self.missing_field > 0"}
        ]));
        let obj = json!({"replicas": 1, "name": "app"});
        let errors = validate(&schema, &obj, None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, ErrorKind::EvaluationError);
    }
}
