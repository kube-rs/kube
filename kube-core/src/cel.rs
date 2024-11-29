//! CEL validation for CRDs

use std::str::FromStr;

#[cfg(feature = "schema")] use schemars::schema::Schema;
use serde::{Deserialize, Serialize};

/// Rule is a CEL validation rule for the CRD field
#[derive(Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// rule represents the expression which will be evaluated by CEL.
    /// The `self` variable in the CEL expression is bound to the scoped value.
    pub rule: String,
    /// message represents CEL validation message for the provided type
    /// If unset, the message is "failed rule: {Rule}".
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    /// fieldPath represents the field path returned when the validation fails.
    /// It must be a relative JSON path, scoped to the location of the field in the schema
    pub field_path: Option<String>,
    /// reason is a machine-readable value providing more detail about why a field failed the validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

impl From<&str> for Rule {
    fn from(value: &str) -> Self {
        Self {
            rule: value.into(),
            ..Default::default()
        }
    }
}

impl From<(&str, &str)> for Rule {
    fn from((rule, msg): (&str, &str)) -> Self {
        Self {
            rule: rule.into(),
            message: Some(msg.into()),
            ..Default::default()
        }
    }
}
/// Message represents CEL validation message for the provided type
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Message {
    /// Message represents the message displayed when validation fails. The message is required if the Rule contains
    /// line breaks. The message must not contain line breaks.
    /// Example:
    /// "must be a URL with the host matching spec.host"
    Message(String),
    /// Expression declares a CEL expression that evaluates to the validation failure message that is returned when this rule fails.
    /// Since messageExpression is used as a failure message, it must evaluate to a string. If messageExpression results in a runtime error, the runtime error is logged, and the validation failure message is produced
    /// as if the messageExpression field were unset. If messageExpression evaluates to an empty string, a string with only spaces, or a string
    /// that contains line breaks, then the validation failure message will also be produced as if the messageExpression field were unset, and
    /// the fact that messageExpression produced an empty string/string with only spaces/string with line breaks will be logged.
    /// messageExpression has access to all the same variables as the rule; the only difference is the return type.
    /// Example:
    /// "x must be less than max ("+string(self.max)+")"
    #[serde(rename = "messageExpression")]
    Expression(String),
}

impl From<&str> for Message {
    fn from(value: &str) -> Self {
        Message::Message(value.to_string())
    }
}

/// Reason is a machine-readable value providing more detail about why a field failed the validation.
///
/// More in [docs](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#field-reason)
#[derive(Serialize, Deserialize, Clone)]
pub enum Reason {
    /// FieldValueInvalid is used to report malformed values (e.g. failed regex
    /// match, too long, out of bounds).
    FieldValueInvalid,
    /// FieldValueForbidden is used to report valid (as per formatting rules)
    /// values which would be accepted under some conditions, but which are not
    /// permitted by the current conditions (such as security policy).
    FieldValueForbidden,
    /// FieldValueRequired is used to report required values that are not
    /// provided (e.g. empty strings, null values, or empty arrays).
    FieldValueRequired,
    /// FieldValueDuplicate is used to report collisions of values that must be
    /// unique (e.g. unique IDs).
    FieldValueDuplicate,
}

impl FromStr for Reason {
    type Err = serde_json::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_str(s)
    }
}

/// Validate takes schema and applies a set of validation rules to it. The rules are stored
/// under the "x-kubernetes-validations".
///
/// ```rust
/// use schemars::schema::Schema;
/// use kube::core::{Rule, Reason, Message, validate};
///
/// let mut schema = Schema::Object(Default::default());
/// let rules = vec![Rule{
///     rule: "self.spec.host == self.url.host".into(),
///     message: Some("must be a URL with the host matching spec.host".into()),
///     field_path: Some("spec.host".into()),
///     ..Default::default()
/// }];
/// let schema = validate(&mut schema, rules)?;
/// assert_eq!(
///     serde_json::to_string(&schema).unwrap(),
///     r#"{"x-kubernetes-validations":[{"fieldPath":"spec.host","message":"must be a URL with the host matching spec.host","rule":"self.spec.host == self.url.host"}]}"#,
/// );
/// # Ok::<(), serde_json::Error>(())
///```
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub fn validate(s: &mut Schema, rules: Vec<Rule>) -> Result<Schema, serde_json::Error> {
    let rules = serde_json::to_value(rules)?;
    match s {
        Schema::Bool(_) => (),
        Schema::Object(schema_object) => {
            schema_object
                .extensions
                .insert("x-kubernetes-validations".into(), rules);
        }
    };

    Ok(s.clone())
}
