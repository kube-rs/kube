//! CEL validation for CRDs

use std::str::FromStr;

#[cfg(feature = "schema")] use schemars::schema::Schema;
use serde::{Deserialize, Serialize};

/// Rule is a CEL validation rule for the CRD field
#[derive(Default, Serialize, Deserialize, Clone, Debug)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_path: Option<String>,
    /// reason is a machine-readable value providing more detail about why a field failed the validation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<Reason>,
}

impl Rule {
    /// Initialize the rule
    ///
    /// ```rust
    /// use kube_core::Rule;
    /// let r = Rule::new("self == oldSelf");
    ///
    /// assert_eq!(r.rule, "self == oldSelf".to_string())
    /// ```
    pub fn new(rule: impl Into<String>) -> Self {
        Self {
            rule: rule.into(),
            ..Default::default()
        }
    }

    /// Set the rule message.
    ///
    /// use kube_core::Rule;
    /// ```rust
    /// use kube_core::{Rule, Message};
    ///
    /// let r = Rule::new("self == oldSelf").message("is immutable");
    /// assert_eq!(r.rule, "self == oldSelf".to_string());
    /// assert_eq!(r.message, Some(Message::Message("is immutable".to_string())));
    /// ```
    pub fn message(mut self, message: impl Into<Message>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Set the failure reason.
    ///
    /// use kube_core::Rule;
    /// ```rust
    /// use kube_core::{Rule, Reason};
    ///
    /// let r = Rule::new("self == oldSelf").reason(Reason::default());
    /// assert_eq!(r.rule, "self == oldSelf".to_string());
    /// assert_eq!(r.reason, Some(Reason::FieldValueInvalid));
    /// ```
    pub fn reason(mut self, reason: impl Into<Reason>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    /// Set the failure field_path.
    ///
    /// use kube_core::Rule;
    /// ```rust
    /// use kube_core::Rule;
    ///
    /// let r = Rule::new("self == oldSelf").field_path("obj.field");
    /// assert_eq!(r.rule, "self == oldSelf".to_string());
    /// assert_eq!(r.field_path, Some("obj.field".to_string()));
    /// ```
    pub fn field_path(mut self, field_path: impl Into<String>) -> Self {
        self.field_path = Some(field_path.into());
        self
    }
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq)]
pub enum Reason {
    /// FieldValueInvalid is used to report malformed values (e.g. failed regex
    /// match, too long, out of bounds).
    #[default]
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
/// on the top level under the "x-kubernetes-validations".
///
/// ```rust
/// use schemars::schema::Schema;
/// use kube::core::{Rule, Reason, Message, validate};
///
/// let mut schema = Schema::Object(Default::default());
/// let rules = &[Rule{
///     rule: "self.spec.host == self.url.host".into(),
///     message: Some("must be a URL with the host matching spec.host".into()),
///     field_path: Some("spec.host".into()),
///     ..Default::default()
/// }];
/// validate(&mut schema, rules)?;
/// assert_eq!(
///     serde_json::to_string(&schema).unwrap(),
///     r#"{"x-kubernetes-validations":[{"fieldPath":"spec.host","message":"must be a URL with the host matching spec.host","rule":"self.spec.host == self.url.host"}]}"#,
/// );
/// # Ok::<(), serde_json::Error>(())
///```
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub fn validate(s: &mut Schema, rules: &[Rule]) -> Result<(), serde_json::Error> {
    match s {
        Schema::Bool(_) => (),
        Schema::Object(schema_object) => {
            schema_object
                .extensions
                .insert("x-kubernetes-validations".into(), serde_json::to_value(rules)?);
        }
    };
    Ok(())
}

/// Validate property mutates property under property_index of the schema
/// with the provided set of validation rules.
///
/// ```rust
/// use schemars::JsonSchema;
/// use kube::core::{Rule, validate_property};
///
/// #[derive(JsonSchema)]
/// struct MyStruct {
///     field: Option<String>,
/// }
///
/// let gen = &mut schemars::gen::SchemaSettings::openapi3().into_generator();
/// let mut schema = MyStruct::json_schema(gen);
/// let rules = &[Rule::new("self != oldSelf")];
/// validate_property(&mut schema, 0, rules)?;
/// assert_eq!(
///     serde_json::to_string(&schema).unwrap(),
///     r#"{"type":"object","properties":{"field":{"type":"string","nullable":true,"x-kubernetes-validations":[{"rule":"self != oldSelf"}]}}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
///```
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub fn validate_property(
    s: &mut Schema,
    property_index: usize,
    rules: &[Rule],
) -> Result<(), serde_json::Error> {
    match s {
        Schema::Bool(_) => (),
        Schema::Object(schema_object) => {
            let obj = schema_object.object();
            for (n, (_, schema)) in obj.properties.iter_mut().enumerate() {
                if n == property_index {
                    return validate(schema, rules);
                }
            }
        }
    };

    Ok(())
}

/// Merge schema properties in order to pass overrides or extension properties from the other schema.
///
/// ```rust
/// use schemars::JsonSchema;
/// use kube::core::{Rule, merge_properties};
///
/// #[derive(JsonSchema)]
/// struct MyStruct {
///     a: Option<bool>,
/// }
///
/// #[derive(JsonSchema)]
/// struct MySecondStruct {
///     a: bool,
///     b: Option<bool>,
/// }
/// let gen = &mut schemars::gen::SchemaSettings::openapi3().into_generator();
/// let mut first = MyStruct::json_schema(gen);
/// let mut second = MySecondStruct::json_schema(gen);
/// merge_properties(&mut first, &mut second);
///
/// assert_eq!(
///     serde_json::to_string(&first).unwrap(),
///     r#"{"type":"object","properties":{"a":{"type":"boolean"},"b":{"type":"boolean","nullable":true}}}"#
/// );
/// # Ok::<(), serde_json::Error>(())
#[cfg(feature = "schema")]
#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
pub fn merge_properties(s: &mut Schema, merge: &mut Schema) {
    match s {
        schemars::schema::Schema::Bool(_) => (),
        schemars::schema::Schema::Object(schema_object) => {
            let obj = schema_object.object();
            for (k, v) in &merge.clone().into_object().object().properties {
                obj.properties.insert(k.clone(), v.clone());
            }
        }
    }
}
