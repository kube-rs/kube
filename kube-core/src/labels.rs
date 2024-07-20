//! Type safe label selector logic
use core::fmt;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, LabelSelectorRequirement};
use serde::{Deserialize, Serialize};
use std::{
    cmp::PartialEq,
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    iter::FromIterator,
    option::IntoIter,
};
use thiserror::Error;

mod private {
    pub trait Sealed {}
    impl Sealed for super::Expression {}
    impl Sealed for super::Selector {}
}

#[derive(Debug, Error)]
#[error("failed to parse value as expression: {0}")]
/// Indicates failure of conversion to Expression
pub struct ParseExpressionError(pub String);

// local type aliases
type Expressions = Vec<Expression>;

/// Selector extension trait for querying selector-like objects
pub trait SelectorExt: private::Sealed {
    /// Collection type to compare with self
    type Search;

    /// Perform a match check on the arbitrary components like labels
    ///
    /// ```
    /// use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
    /// use kube::core::{SelectorExt, Selector};
    /// # use std::collections::BTreeMap;
    ///
    /// let selector: Selector = LabelSelector::default().try_into()?;
    /// let search = BTreeMap::from([("app".to_string(), "myapp".to_string())]);
    /// selector.matches(&search);
    /// # Ok::<(), kube_core::ParseExpressionError>(())
    /// ```
    fn matches(&self, on: &Self::Search) -> bool;
}

/// A selector expression with existing operations
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Expression {
    /// Key exists and in set:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::In("foo".into(), ["bar".into(), "baz".into()].into());
    /// assert_eq!(exp.to_string(), "foo in (bar,baz)");
    /// let exp = Expression::In("foo".into(), ["bar".into(), "baz".into()].into_iter().collect());
    /// assert_eq!(exp.to_string(), "foo in (bar,baz)");
    /// ```
    In(String, BTreeSet<String>),

    /// Key does not exists or not in set:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::NotIn("foo".into(), ["bar".into(), "baz".into()].into());
    /// assert_eq!(exp.to_string(), "foo notin (bar,baz)");
    /// let exp = Expression::NotIn("foo".into(), ["bar".into(), "baz".into()].into_iter().collect());
    /// assert_eq!(exp.to_string(), "foo notin (bar,baz)");
    /// ```
    NotIn(String, BTreeSet<String>),

    /// Key exists and is equal:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::Equal("foo".into(), "bar".into());
    /// assert_eq!(exp.to_string(), "foo=bar")
    /// ```
    Equal(String, String),

    /// Key does not exists or is not equal:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::NotEqual("foo".into(), "bar".into());
    /// assert_eq!(exp.to_string(), "foo!=bar")
    /// ```
    NotEqual(String, String),

    /// Key exists:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::Exists("foo".into());
    /// assert_eq!(exp.to_string(), "foo")
    /// ```
    Exists(String),

    /// Key does not exist:
    ///
    /// ```
    /// # use kube_core::Expression;
    /// let exp = Expression::DoesNotExist("foo".into());
    /// assert_eq!(exp.to_string(), "!foo")
    /// ```
    DoesNotExist(String),
}

/// Perform selection on a list of expressions
///
/// Can be injected into [`WatchParams`](crate::params::WatchParams::labels_from) or [`ListParams`](crate::params::ListParams::labels_from).
#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
pub struct Selector(Expressions);

impl Selector {
    /// Create a selector from a vector of expressions
    fn from_expressions(exprs: Expressions) -> Self {
        Self(exprs)
    }

    /// Create a selector from a map of key=value label matches
    fn from_map(map: BTreeMap<String, String>) -> Self {
        Self(map.into_iter().map(|(k, v)| Expression::Equal(k, v)).collect())
    }

    /// Indicates whether this label selector matches everything
    pub fn selects_all(&self) -> bool {
        self.0.is_empty()
    }

    /// Extend the list of expressions for the selector
    ///
    /// ```
    /// use kube::core::{Selector, Expression};
    /// use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
    ///
    /// let mut selector = Selector::default();
    ///
    /// // Extend from expressions:
    /// selector.extend(Expression::Equal("environment".into(), "production".into()));
    /// selector.extend([Expression::Exists("bar".into()), Expression::Exists("foo".into())].into_iter());
    ///
    /// // Extend from native selectors:
    /// let label_selector: Selector = LabelSelector::default().try_into()?;
    /// selector.extend(label_selector);
    /// # Ok::<(), kube_core::ParseExpressionError>(())
    /// ```
    pub fn extend(&mut self, exprs: impl IntoIterator<Item = Expression>) -> &mut Self {
        self.0.extend(exprs);
        self
    }
}

impl SelectorExt for Selector {
    type Search = BTreeMap<String, String>;

    /// Perform a match check on the resource labels
    fn matches(&self, labels: &BTreeMap<String, String>) -> bool {
        for expr in self.0.iter() {
            if !expr.matches(labels) {
                return false;
            }
        }
        true
    }
}

impl SelectorExt for Expression {
    type Search = BTreeMap<String, String>;

    fn matches(&self, labels: &BTreeMap<String, String>) -> bool {
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
        }
    }
}

impl Display for Expression {
    /// Perform conversion to string
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::In(key, values) => {
                write!(
                    f,
                    "{key} in ({})",
                    values.iter().cloned().collect::<Vec<_>>().join(",")
                )
            }
            Expression::NotIn(key, values) => {
                write!(
                    f,
                    "{key} notin ({})",
                    values.iter().cloned().collect::<Vec<_>>().join(",")
                )
            }
            Expression::Equal(key, value) => write!(f, "{key}={value}"),
            Expression::NotEqual(key, value) => write!(f, "{key}!={value}"),
            Expression::Exists(key) => write!(f, "{key}"),
            Expression::DoesNotExist(key) => write!(f, "!{key}"),
        }
    }
}

impl Display for Selector {
    /// Convert a selector to a string for the API
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let selectors: Vec<String> = self.0.iter().map(|e| e.to_string()).collect();
        write!(f, "{}", selectors.join(","))
    }
}
// convenience conversions for Selector and Expression

impl IntoIterator for Expression {
    type IntoIter = IntoIter<Self::Item>;
    type Item = Self;

    fn into_iter(self) -> Self::IntoIter {
        Some(self).into_iter()
    }
}

impl IntoIterator for Selector {
    type IntoIter = std::vec::IntoIter<Self::Item>;
    type Item = Expression;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, String)> for Selector {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self::from_map(iter.into_iter().collect())
    }
}

impl FromIterator<(&'static str, &'static str)> for Selector {
    /// ```
    /// use kube_core::{Selector, Expression};
    ///
    /// let sel: Selector = Some(("foo", "bar")).into_iter().collect();
    /// let equal: Selector = Expression::Equal("foo".into(), "bar".into()).into();
    /// assert_eq!(sel, equal)
    /// ```
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

impl TryFrom<LabelSelector> for Selector {
    type Error = ParseExpressionError;

    fn try_from(value: LabelSelector) -> Result<Self, Self::Error> {
        let expressions = match value.match_expressions {
            Some(requirements) => requirements.into_iter().map(TryInto::try_into).collect(),
            None => Ok(vec![]),
        }?;
        let mut equality: Selector = value
            .match_labels
            .map(|labels| labels.into_iter().collect())
            .unwrap_or_default();
        equality.extend(expressions);
        Ok(equality)
    }
}

impl TryFrom<LabelSelectorRequirement> for Expression {
    type Error = ParseExpressionError;

    fn try_from(requirement: LabelSelectorRequirement) -> Result<Self, Self::Error> {
        let key = requirement.key;
        let values = requirement.values.map(|values| values.into_iter().collect());
        match requirement.operator.as_str() {
            "In" => match values {
                Some(values) => Ok(Expression::In(key, values)),
                None => Err(ParseExpressionError(
                    "Expected values for In operator, got none".into(),
                )),
            },
            "NotIn" => match values {
                Some(values) => Ok(Expression::NotIn(key, values)),
                None => Err(ParseExpressionError(
                    "Expected values for In operator, got none".into(),
                )),
            },
            "Exists" => Ok(Expression::Exists(key)),
            "DoesNotExist" => Ok(Expression::DoesNotExist(key)),
            _ => Err(ParseExpressionError("Invalid expression operator".into())),
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
        for (selector, label_selector, labels, matches, msg) in &[
            (
                Selector::default(),
                LabelSelector::default(),
                Default::default(),
                true,
                "empty match",
            ),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                LabelSelector {
                    match_labels: Some([("foo".into(), "bar".into())].into()),
                    match_expressions: Default::default(),
                },
                [("foo".to_string(), "bar".to_string())].into(),
                true,
                "exact label match",
            ),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                LabelSelector {
                    match_labels: Some([("foo".to_string(), "bar".to_string())].into()),
                    match_expressions: None,
                },
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
                LabelSelector {
                    match_labels: None,
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "foo".into(),
                        operator: "In".to_string(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: Some([("foo".into(), "bar".into())].into()),
                    match_expressions: None,
                },
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
                LabelSelector {
                    match_labels: None,
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "foo".into(),
                        operator: "NotIn".into(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: None,
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "foo".into(),
                        operator: "In".into(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: None,
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "foo".into(),
                        operator: "NotIn".into(),
                        values: Some(vec!["quux".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: None,
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "foo".into(),
                        operator: "NotIn".into(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: Some([("foo".into(), "bar".into())].into()),
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "bah".into(),
                        operator: "In".into(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
                LabelSelector {
                    match_labels: Some([("foo".into(), "bar".into())].into()),
                    match_expressions: Some(vec![LabelSelectorRequirement {
                        key: "bah".into(),
                        operator: "In".into(),
                        values: Some(vec!["bar".into()]),
                    }]),
                },
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
            let converted: LabelSelector = selector.clone().into();
            assert_eq!(&converted, label_selector);
            let converted_selector: Selector = converted.try_into().unwrap();
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
        .try_into()
        .unwrap();
        assert!(selector.matches(&[("foo".into(), "bar".into())].into()));
        assert!(!selector.matches(&Default::default()));
    }

    #[test]
    fn test_to_string() {
        let selector = Selector(vec![
            Expression::In("foo".into(), ["bar".into(), "baz".into()].into()),
            Expression::NotIn("foo".into(), ["bar".into(), "baz".into()].into()),
            Expression::Equal("foo".into(), "bar".into()),
            Expression::NotEqual("foo".into(), "bar".into()),
            Expression::Exists("foo".into()),
            Expression::DoesNotExist("foo".into()),
        ])
        .to_string();

        assert_eq!(
            selector,
            "foo in (bar,baz),foo notin (bar,baz),foo=bar,foo!=bar,foo,!foo"
        )
    }
}
