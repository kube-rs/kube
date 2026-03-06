//! Compilation of Kubernetes CRD `x-kubernetes-validations` rules into CEL programs.
//!
//! This module parses validation rules from CRD schemas and compiles them into
//! [`cel::Program`] instances that can be evaluated against resource data.

use std::collections::HashMap;

use cel::{ParseErrors, Program};

use super::values::SchemaFormat;

/// A single CRD `x-kubernetes-validations` rule.
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// The CEL expression to evaluate.
    pub rule: String,
    /// Static error message returned when validation fails.
    #[serde(default)]
    pub message: Option<String>,
    /// CEL expression that produces a dynamic error message.
    #[serde(default)]
    pub message_expression: Option<String>,
    /// Machine-readable reason for the validation failure (e.g. "FieldValueForbidden").
    #[serde(default)]
    pub reason: Option<String>,
    /// JSONPath to the field that caused the failure.
    #[serde(default)]
    pub field_path: Option<String>,
    /// Whether `oldSelf` is optional. When `true`, transition rules are
    /// evaluated even on create (with `oldSelf` bound to null).
    #[serde(default)]
    pub optional_old_self: Option<bool>,
}

/// The result of successfully compiling a [`Rule`].
#[derive(Debug)]
pub struct CompilationResult {
    /// The compiled CEL program.
    pub program: Program,
    /// The original rule that was compiled.
    pub rule: Rule,
    /// Whether the rule references `oldSelf` (transition rule).
    pub is_transition_rule: bool,
    /// Pre-compiled `messageExpression` program (if present and valid).
    /// `None` if no `messageExpression` was specified or if it failed to compile.
    pub message_program: Option<Program>,
}

/// Errors that can occur during rule compilation.
#[derive(Debug)]
pub enum CompilationError {
    /// CEL expression failed to parse.
    Parse {
        /// The original CEL expression that failed to compile.
        rule: String,
        /// The parse errors reported by the CEL compiler.
        source: ParseErrors,
    },
    /// JSON value could not be deserialized into a [`Rule`].
    InvalidRule(serde_json::Error),
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilationError::Parse { rule, source } => {
                write!(f, "failed to compile CEL rule \"{rule}\": {source}")
            }
            CompilationError::InvalidRule(err) => {
                write!(f, "invalid rule definition: {err}")
            }
        }
    }
}

impl std::error::Error for CompilationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CompilationError::Parse { source, .. } => Some(source),
            CompilationError::InvalidRule(err) => Some(err),
        }
    }
}

/// Compile a single [`Rule`] into a [`CompilationResult`].
///
/// Returns [`CompilationError::Parse`] if the CEL expression is invalid.
pub(crate) fn compile_rule(rule: &Rule) -> Result<CompilationResult, CompilationError> {
    let program = Program::compile(&rule.rule).map_err(|e| CompilationError::Parse {
        rule: rule.rule.clone(),
        source: e,
    })?;
    let is_transition_rule = program.references().has_variable("oldSelf");

    // Best-effort: compile messageExpression if present, ignore failures
    let message_program = rule
        .message_expression
        .as_deref()
        .and_then(|expr| Program::compile(expr).ok());

    Ok(CompilationResult {
        program,
        rule: rule.clone(),
        is_transition_rule,
        message_program,
    })
}

/// Extract `x-kubernetes-validations` rules from a schema node and compile them.
///
/// If the schema has no `x-kubernetes-validations` key or it is not an array,
/// returns an empty `Vec`. Each rule is compiled independently — failures in one
/// rule do not prevent others from compiling.
pub(crate) fn compile_schema_validations(
    schema: &serde_json::Value,
) -> Vec<Result<CompilationResult, CompilationError>> {
    let rules = match schema.get("x-kubernetes-validations") {
        Some(serde_json::Value::Array(arr)) => arr,
        _ => return Vec::new(),
    };

    rules
        .iter()
        .map(|raw| {
            let rule: Rule = serde_json::from_value(raw.clone()).map_err(CompilationError::InvalidRule)?;
            compile_rule(&rule)
        })
        .collect()
}

/// A pre-compiled schema tree. Compile once with [`compile_schema`], then
/// validate many objects via [`Validator::validate_compiled`](super::validation::Validator::validate_compiled).
///
/// # Note
///
/// `CompiledSchema` is not `Clone` because [`cel::Program`] is `!Clone`.
/// Wrap in [`Arc`](std::sync::Arc) for shared ownership across threads.
#[derive(Debug)]
pub struct CompiledSchema {
    /// Compiled validation rules at this schema node.
    pub validations: Vec<Result<CompilationResult, CompilationError>>,
    /// Compiled child property schemas.
    pub properties: HashMap<String, CompiledSchema>,
    /// Compiled array items schema.
    pub items: Option<Box<CompiledSchema>>,
    /// Compiled additionalProperties schema.
    pub additional_properties: Option<Box<CompiledSchema>>,
    /// The `format` hint from the schema (e.g., `date-time`, `duration`).
    pub format: SchemaFormat,
}

impl CompiledSchema {
    /// Returns references to all compilation errors in this node's validations.
    #[must_use]
    pub fn compilation_errors(&self) -> Vec<&CompilationError> {
        self.validations.iter().filter_map(|r| r.as_ref().err()).collect()
    }

    /// Returns `true` if any validation rule at this node failed to compile.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.validations.iter().any(|r| r.is_err())
    }
}

/// Recursively compile all `x-kubernetes-validations` rules in a schema tree.
///
/// Returns a [`CompiledSchema`] that can be reused across multiple validation
/// calls, avoiding repeated compilation.
#[must_use]
pub fn compile_schema(schema: &serde_json::Value) -> CompiledSchema {
    let validations = compile_schema_validations(schema);

    let mut properties = HashMap::new();
    if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
        for (name, prop_schema) in props {
            properties.insert(name.clone(), compile_schema(prop_schema));
        }
    }

    let items = schema.get("items").map(|s| Box::new(compile_schema(s)));

    let additional_properties = schema
        .get("additionalProperties")
        .filter(|a| a.is_object())
        .map(|s| Box::new(compile_schema(s)));

    let format = SchemaFormat::from_schema(schema);

    CompiledSchema {
        validations,
        properties,
        items,
        additional_properties,
        format,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn compile_simple_rule() {
        let rule = Rule {
            rule: "self.replicas >= 0".into(),
            message: None,
            message_expression: None,
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        assert!(!result.is_transition_rule);
    }

    #[test]
    fn detect_transition_rule() {
        let rule = Rule {
            rule: "self.replicas >= oldSelf.replicas".into(),
            message: None,
            message_expression: None,
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        assert!(result.is_transition_rule);
    }

    #[test]
    fn detect_non_transition_rule() {
        let rule = Rule {
            rule: "self.replicas > 0".into(),
            message: None,
            message_expression: None,
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        assert!(!result.is_transition_rule);
    }

    #[test]
    fn parse_error_on_invalid_cel() {
        let rule = Rule {
            rule: "self.replicas >=".into(),
            message: None,
            message_expression: None,
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let err = compile_rule(&rule).unwrap_err();
        assert!(matches!(err, CompilationError::Parse { .. }));
        // Display should contain the rule text
        let msg = err.to_string();
        assert!(msg.contains("self.replicas >="));
    }

    #[test]
    fn deserialize_rule_all_fields() {
        let raw = json!({
            "rule": "self.x > 0",
            "message": "x must be positive",
            "messageExpression": "\"x is \" + string(self.x)",
            "reason": "FieldValueInvalid",
            "fieldPath": ".spec.x",
            "optionalOldSelf": true
        });
        let rule: Rule = serde_json::from_value(raw).unwrap();
        assert_eq!(rule.rule, "self.x > 0");
        assert_eq!(rule.message.as_deref(), Some("x must be positive"));
        assert_eq!(
            rule.message_expression.as_deref(),
            Some("\"x is \" + string(self.x)")
        );
        assert_eq!(rule.reason.as_deref(), Some("FieldValueInvalid"));
        assert_eq!(rule.field_path.as_deref(), Some(".spec.x"));
        assert_eq!(rule.optional_old_self, Some(true));
    }

    #[test]
    fn deserialize_rule_minimal() {
        let raw = json!({"rule": "self.x > 0"});
        let rule: Rule = serde_json::from_value(raw).unwrap();
        assert_eq!(rule.rule, "self.x > 0");
        assert!(rule.message.is_none());
        assert!(rule.message_expression.is_none());
        assert!(rule.reason.is_none());
        assert!(rule.field_path.is_none());
        assert!(rule.optional_old_self.is_none());
    }

    #[test]
    fn schema_validations_extracts_and_compiles() {
        let schema = json!({
            "type": "object",
            "x-kubernetes-validations": [
                {"rule": "self.replicas >= 0", "message": "must be non-negative"},
                {"rule": "self.name.size() > 0"}
            ]
        });
        let results = compile_schema_validations(&schema);
        assert_eq!(results.len(), 2);
        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[test]
    fn schema_validations_no_key() {
        let schema = json!({"type": "object"});
        let results = compile_schema_validations(&schema);
        assert!(results.is_empty());
    }

    #[test]
    fn schema_validations_empty_array() {
        let schema = json!({
            "x-kubernetes-validations": []
        });
        let results = compile_schema_validations(&schema);
        assert!(results.is_empty());
    }

    #[test]
    fn message_expression_compiled() {
        let rule = Rule {
            rule: "self.x > 0".into(),
            message: Some("x must be positive".into()),
            message_expression: Some("'x is ' + string(self.x)".into()),
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        assert!(result.message_program.is_some());
    }

    #[test]
    fn message_expression_invalid_ignored() {
        let rule = Rule {
            rule: "self.x > 0".into(),
            message: Some("fallback".into()),
            message_expression: Some("invalid >=".into()),
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        // Invalid messageExpression is silently ignored
        assert!(result.message_program.is_none());
    }

    #[test]
    fn message_expression_none() {
        let rule = Rule {
            rule: "self.x > 0".into(),
            message: None,
            message_expression: None,
            reason: None,
            field_path: None,
            optional_old_self: None,
        };
        let result = compile_rule(&rule).unwrap();
        assert!(result.message_program.is_none());
    }

    #[test]
    fn compile_schema_tree() {
        let schema = json!({
            "type": "object",
            "x-kubernetes-validations": [{"rule": "has(self.spec)"}],
            "properties": {
                "spec": {
                    "type": "object",
                    "x-kubernetes-validations": [{"rule": "self.replicas >= 0"}],
                    "properties": {
                        "replicas": {"type": "integer"}
                    }
                }
            }
        });
        let compiled = compile_schema(&schema);
        assert_eq!(compiled.validations.len(), 1);
        assert!(compiled.properties.contains_key("spec"));
        let spec = &compiled.properties["spec"];
        assert_eq!(spec.validations.len(), 1);
        assert!(spec.properties.contains_key("replicas"));
    }

    #[test]
    fn compile_schema_with_items() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "x-kubernetes-validations": [{"rule": "self.name.size() > 0"}]
            }
        });
        let compiled = compile_schema(&schema);
        assert!(compiled.items.is_some());
        assert_eq!(compiled.items.as_ref().unwrap().validations.len(), 1);
    }

    #[test]
    fn compile_schema_empty() {
        let schema = json!({"type": "object"});
        let compiled = compile_schema(&schema);
        assert!(compiled.validations.is_empty());
        assert!(compiled.properties.is_empty());
        assert!(compiled.items.is_none());
        assert!(compiled.additional_properties.is_none());
    }

    #[test]
    fn schema_validations_partial_errors() {
        let schema = json!({
            "x-kubernetes-validations": [
                {"rule": "self.x > 0"},
                {"rule": "self.y >="},
                {"rule": "self.z == true"}
            ]
        });
        let results = compile_schema_validations(&schema);
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }

    #[test]
    fn compilation_errors_method() {
        let schema = json!({
            "x-kubernetes-validations": [
                {"rule": "self.x > 0"},
                {"rule": "self.y >="},
                {"rule": "self.z == true"}
            ]
        });
        let compiled = compile_schema(&schema);
        let errors = compiled.compilation_errors();
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], CompilationError::Parse { .. }));
        assert!(compiled.has_errors());
    }

    #[test]
    fn compilation_errors_empty_when_all_valid() {
        let schema = json!({
            "x-kubernetes-validations": [
                {"rule": "self.x > 0"},
                {"rule": "self.z == true"}
            ]
        });
        let compiled = compile_schema(&schema);
        assert!(compiled.compilation_errors().is_empty());
        assert!(!compiled.has_errors());
    }
}
