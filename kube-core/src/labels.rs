#![allow(missing_docs)]
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use serde::{Deserialize, Serialize};
use std::{
    cmp::PartialEq,
    collections::{BTreeMap, BTreeSet},
    iter::FromIterator,
};

// local type aliases
type Map = BTreeMap<String, String>;
type Expressions = Vec<Expression>;

/// A selector expression with existing operations
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Expression {
    In(String, BTreeSet<String>),
    NotIn(String, BTreeSet<String>),
    Equal(String, String),
    NotEqual(String, String),
    Exists(String),
    DoesNotExist(String),
    Invalid,
}

/// Perform selection on a list of expressions
#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct Selector(Expressions);

impl Selector {
    /// Create a selector from a vector of expressions
    fn from_expressions(exprs: Expressions) -> Self {
        Self(exprs)
    }

    /// Create a selector from a map of key=value label matches
    fn from_map(map: Map) -> Self {
        Self(map.into_iter().map(|(k, v)| Expression::Equal(k, v)).collect())
    }

    /// Convert a selector to a string for the API
    pub fn to_selector_string(&self) -> String {
        let selectors: Vec<String> = self
            .0
            .iter()
            .filter(|&e| e != &Expression::Invalid)
            .map(|e| e.to_string())
            .collect();
        selectors.join(",")
    }

    /// Indicates whether this label selector matches all pods
    pub fn selects_all(&self) -> bool {
        self.0.is_empty()
    }

    pub fn matches(&self, labels: &Map) -> bool {
        for expr in self.0.iter() {
            if !expr.matches(labels) {
                return false;
            }
        }
        true
    }
}

// === Expression ===

impl Expression {
    /// Perform conversion to string
    pub fn to_string(&self) -> String {
        match self {
            Expression::In(key, values) => {
                format!(
                    "{key} in ({})",
                    values.into_iter().cloned().collect::<Vec<_>>().join(",")
                )
            }
            Expression::NotIn(key, values) => {
                format!(
                    "{key} notin ({})",
                    values.into_iter().cloned().collect::<Vec<_>>().join(",")
                )
            }
            Expression::Equal(key, value) => format!("{key}={value}"),
            Expression::NotEqual(key, value) => format!("{key}!={value}"),
            Expression::Exists(key) => format!("{key}"),
            Expression::DoesNotExist(key) => format!("!{key}"),
            Expression::Invalid => "".into(),
        }
    }

    fn matches(&self, labels: &Map) -> bool {
        match self {
            Expression::In(key, values) => match labels.get(key) {
                Some(v) => values.contains(v),
                None => false,
            },
            Expression::NotIn(key, values) => match labels.get(key) {
                Some(v) => !values.contains(v),
                None => true,
            },
            Expression::Exists(key) => labels.contains_key(key),
            Expression::DoesNotExist(key) => !labels.contains_key(key),
            Expression::Equal(key, value) => labels.get(key) == Some(value),
            Expression::NotEqual(key, value) => labels.get(key) != Some(value),
            Expression::Invalid => false,
        }
    }
}


// convenience conversions for Selector

impl FromIterator<(String, String)> for Selector {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self::from_map(iter.into_iter().collect())
    }
}

impl FromIterator<(&'static str, &'static str)> for Selector {
    fn from_iter<T: IntoIterator<Item = (&'static str, &'static str)>>(iter: T) -> Self {
        Self::from_map(
            iter.into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        )
    }
}

impl FromIterator<Expression> for Selector {
    fn from_iter<T: IntoIterator<Item = Expression>>(iter: T) -> Self {
        Self::from_expressions(iter.into_iter().collect())
    }
}

impl From<Expression> for Selector {
    fn from(value: Expression) -> Self {
        Self(vec![value])
    }
}

impl From<LabelSelector> for Selector {
    fn from(value: LabelSelector) -> Self {
        let expressions = match value.match_expressions {
            Some(requirements) => requirements.into_iter().map(Into::into).collect(),
            None => vec![],
        };
        let mut equality: Selector = value
            .match_labels
            .and_then(|labels| Some(labels.into_iter().collect()))
            .unwrap_or_default();
        equality.0.extend(expressions);
        equality
    }
}

impl From<LabelSelectorRequirement> for Expression {
    fn from(requirement: LabelSelectorRequirement) -> Self {
        let key = requirement.key;
        let values = requirement.values.map(|values| values.into_iter().collect());
        match requirement.operator.as_str() {
            "In" => match values {
                Some(values) => Expression::In(key, values),
                None => Expression::Invalid,
            },
            "NotIn" => match values {
                Some(values) => Expression::NotIn(key, values),
                None => Expression::Invalid,
            },
            "Exists" => Expression::Exists(key),
            "DoesNotExist" => Expression::DoesNotExist(key),
            _ => Expression::Invalid,
        }
    }
}

impl From<Selector> for LabelSelector {
    fn from(value: Selector) -> Self {
        let mut equality = vec![];
        let mut expressions = vec![];
        for expr in value.0 {
            match expr {
                Expression::In(key, values) => expressions.push(LabelSelectorRequirement {
                    key,
                    operator: "In".into(),
                    values: Some(values.into_iter().collect()),
                }),
                Expression::NotIn(key, values) => expressions.push(LabelSelectorRequirement {
                    key,
                    operator: "NotIn".into(),
                    values: Some(values.into_iter().collect()),
                }),
                Expression::Equal(key, value) => equality.push((key, value)),
                Expression::NotEqual(key, value) => expressions.push(LabelSelectorRequirement {
                    key,
                    operator: "NotIn".into(),
                    values: Some(vec![value]),
                }),
                Expression::Exists(key) => expressions.push(LabelSelectorRequirement {
                    key,
                    operator: "Exists".into(),
                    values: None,
                }),
                Expression::DoesNotExist(key) => expressions.push(LabelSelectorRequirement {
                    key,
                    operator: "DoesNotExist".into(),
                    values: None,
                }),
                Expression::Invalid => (),
            }
        }

        LabelSelector {
            match_labels: (!equality.is_empty()).then_some(equality.into_iter().collect()),
            match_expressions: (!expressions.is_empty()).then_some(expressions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn test_raw_matches() {
        for (selector, labels, matches, msg) in &[
            (Selector::default(), Default::default(), true, "empty match"),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                [("foo".to_string(), "bar".to_string())].into(),
                true,
                "exact label match",
            ),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                true,
                "sufficient label match",
            ),
            (
                Selector::from_iter(Some(Expression::In(
                    "foo".into(),
                    Some("bar".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                true,
                "In expression match",
            ),
            (
                Selector::from_iter(Some(Expression::Equal(
                    "foo".into(),
                    Some("bar".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                true,
                "Equal expression match",
            ),
            (
                Selector::from_iter(Some(Expression::NotEqual(
                    "foo".into(),
                    Some("bar".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                false,
                "NotEqual expression match",
            ),
            (
                Selector::from_iter(Some(Expression::In(
                    "foo".into(),
                    Some("bar".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                true,
                "In expression match",
            ),
            (
                Selector::from_iter(Some(Expression::NotIn(
                    "foo".into(),
                    Some("quux".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                true,
                "NotIn expression match",
            ),
            (
                Selector::from_iter(Some(Expression::NotIn(
                    "foo".into(),
                    Some("bar".to_string()).into_iter().collect(),
                ))),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                false,
                "NotIn expression non-match",
            ),
            (
                Selector(vec![
                    Expression::Equal("foo".to_string(), "bar".to_string()),
                    Expression::In("bah".into(), Some("bar".to_string()).into_iter().collect()),
                ]),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "baz".to_string()),
                ]
                .into(),
                false,
                "matches labels but not expressions",
            ),
            (
                Selector(vec![
                    Expression::Equal("foo".to_string(), "bar".to_string()),
                    Expression::In("bah".into(), Some("bar".to_string()).into_iter().collect()),
                ]),
                [
                    ("foo".to_string(), "bar".to_string()),
                    ("bah".to_string(), "bar".to_string()),
                ]
                .into(),
                true,
                "matches both labels and expressions",
            ),
        ] {
            assert_eq!(selector.matches(labels), *matches, "{}", msg);
            let label_selector: LabelSelector = selector.clone().into();
            let converted_selector: Selector = label_selector.into();
            assert_eq!(
                converted_selector.matches(labels),
                *matches,
                "After conversion: {}",
                msg
            );
        }
    }

    #[test]
    fn test_label_selector_matches() {
        let selector: Selector = LabelSelector {
            match_expressions: Some(vec![
                LabelSelectorRequirement {
                    key: "foo".into(),
                    operator: "In".into(),
                    values: Some(vec!["bar".into()]),
                },
                LabelSelectorRequirement {
                    key: "foo".into(),
                    operator: "NotIn".into(),
                    values: Some(vec!["baz".into()]),
                },
                LabelSelectorRequirement {
                    key: "foo".into(),
                    operator: "Exists".into(),
                    values: None,
                },
                LabelSelectorRequirement {
                    key: "baz".into(),
                    operator: "DoesNotExist".into(),
                    values: None,
                },
            ]),
            match_labels: Some([("foo".into(), "bar".into())].into()),
        }
        .into();
        assert!(selector.matches(&[("foo".into(), "bar".into())].into()));
        assert!(!selector.matches(&Default::default()));
    }

    #[test]
    fn test_to_selector_string() {
        let selector = Selector(vec![
            Expression::In("foo".into(), ["bar".into(), "baz".into()].into()),
            Expression::NotIn("foo".into(), ["bar".into(), "baz".into()].into()),
            Expression::Equal("foo".into(), "bar".into()),
            Expression::NotEqual("foo".into(), "bar".into()),
            Expression::Exists("foo".into()),
            Expression::DoesNotExist("foo".into()),
        ])
        .to_selector_string();

        assert_eq!(
            selector,
            "foo in (bar,baz),foo notin (bar,baz),foo=bar,foo!=bar,foo,!foo"
        )
    }
}
