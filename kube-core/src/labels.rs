#![allow(missing_docs)] // for now! prototyping
use serde::{Deserialize, Serialize};
//use schemars::JsonSchema;
use std::{
    cmp::PartialEq,
    collections::{BTreeMap, BTreeSet},
    iter::FromIterator,
};

/// Labels as extracted from a container
/// TODO: users don't get the linkerd label type easily, rethink this...
//#[derive(Clone, Debug, Default)]
//pub struct Labels(Arc<Map>);
// TODO: why is this Arc wrapped?
// TODO: to_string impl

// TODO: add impls on Labels to add in Expressions

// local type aliases
type Map = BTreeMap<String, String>;
type Expressions = Vec<Expression>;

// TODO cfg attr jsonschema?
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Expression {
    key: String,
    operator: Operator,
    values: Option<BTreeSet<String>>,
}

impl Expression {
    /// Create an "key in (values,..)" expression
    pub fn key_in(key: String, values: BTreeSet<String>) -> Self {
        Self {
            key,
            operator: Operator::In,
            values: Some(values),
        }
    }

    // TODO: more builders here

    // need a serializer for this also..
    pub fn to_string(&self) -> String {
        if let Some(values) = &self.values {
            let mut set_str = String::new(); // impl on Values?
            for v in values {
                set_str.push_str(v);
                set_str.push(',');
            }
            // TODO: trailing ,
            format!("{} {:?} ({set_str})", self.key, self.operator)
        } else {
            format!("{} {:?}", self.key, self.operator)
        }
    }
}

/// A selector operator
///
/// TODO: make this smarter? embed values in In/NotIn variants?
#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum Operator {
    In,
    NotIn,
    Exists,
    DoesNotExist,
}

/// Selects a set of pods that expose a server
///
/// The result of `match_labels` and `match_expressions` are ANDed.
#[derive(Clone, Debug, Eq, PartialEq, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selector {
    match_labels: Option<Map>,
    match_expressions: Option<Expressions>,
}

impl Selector {
    #[cfg(test)]
    fn new(labels: Map, exprs: Expressions) -> Self {
        Self {
            match_labels: Some(labels),
            match_expressions: Some(exprs),
        }
    }

    /// Create a selector from a vector of expressions
    fn from_expressions(exprs: Expressions) -> Self {
        Self {
            match_labels: None,
            match_expressions: Some(exprs),
        }
    }

    /// Create a selector from a map of key=value label matches
    fn from_map(map: Map) -> Self {
        Self {
            match_labels: Some(map),
            match_expressions: None,
        }
    }

    /// Convert a selector to a string for the API
    pub fn to_selector_string(&self) -> String {
        let mut sel = String::new();
        if let Some(labels) = &self.match_labels {
            for (k, v) in labels {
                sel.push_str(&k);
                sel.push('=');
                sel.push_str(&v);
                sel.push(',');
            }
        }
        if let Some(exprs) = &self.match_expressions {
            for exp in exprs {
                sel.push_str(&exp.to_string());
                sel.push(',');
            }
        }
        // TODO: trim trailing ','
        sel
    }

    /// Indicates whether this label selector matches all pods
    pub fn selects_all(&self) -> bool {
        match (self.match_labels.as_ref(), self.match_expressions.as_ref()) {
            (None, None) => true,
            (Some(l), None) => l.is_empty(),
            (None, Some(e)) => e.is_empty(),
            (Some(l), Some(e)) => l.is_empty() && e.is_empty(),
        }
    }

    // users don't get the linkerd label type easily, rethink this...
    /*
    pub fn matches(&self, labels: &Labels) -> bool {
        for expr in self.match_expressions.iter().flatten() {
            if !expr.matches(labels.as_ref()) {
                return false;
            }
        }
        if let Some(match_labels) = self.match_labels.as_ref() {
            for (k, v) in match_labels {
                if labels.0.get(k) != Some(v) {
                    return false;
                }
            }
        }
        true
    }
    */
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

/*
// === Labels ===

impl From<Option<Map>> for Labels {
    #[inline]
    fn from(labels: Option<Map>) -> Self {
        labels.unwrap_or_default().into()
    }
}

impl From<Map> for Labels {
    #[inline]
    fn from(labels: Map) -> Self {
        Self(Arc::new(labels))
    }
}

impl AsRef<Map> for Labels {
    #[inline]
    fn as_ref(&self) -> &Map {
        self.0.as_ref()
    }
}

impl PartialEq<Self> for Labels {
    #[inline]
    fn eq(&self, t: &Self) -> bool {
        self.0.as_ref().eq(t.as_ref())
    }
}

impl PartialEq<Option<Map>> for Labels {
    #[inline]
    fn eq(&self, t: &Option<Map>) -> bool {
        match t {
            None => self.0.is_empty(),
            Some(t) => t.eq(self.0.as_ref()),
        }
    }
}

impl FromIterator<(String, String)> for Labels {
    fn from_iter<T: IntoIterator<Item = (String, String)>>(iter: T) -> Self {
        Self(Arc::new(iter.into_iter().collect()))
    }
}

impl FromIterator<(&'static str, &'static str)> for Labels {
    fn from_iter<T: IntoIterator<Item = (&'static str, &'static str)>>(iter: T) -> Self {
        iter.into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }
}*/

// === Expression ===

impl Expression {
    fn matches(&self, labels: &Map) -> bool {
        match (self.operator, &self.key, self.values.as_ref()) {
            (Operator::In, key, Some(values)) => match labels.get(key) {
                Some(v) => values.contains(v),
                None => false,
            },
            (Operator::NotIn, key, Some(values)) => match labels.get(key) {
                Some(v) => !values.contains(v),
                None => true,
            },
            (Operator::Exists, key, None) => labels.contains_key(key),
            (Operator::DoesNotExist, key, None) => !labels.contains_key(key),
            (operator, key, values) => {
                //tracing::warn!(?operator, %key, ?values, "illegal match expression");
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn test_matches() {
        for (selector, labels, matches, msg) in &[
            (Selector::default(), Labels::default(), true, "empty match"),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                Labels::from_iter(Some(("foo", "bar"))),
                true,
                "exact label match",
            ),
            (
                Selector::from_iter(Some(("foo", "bar"))),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "baz")]),
                true,
                "sufficient label match",
            ),
            (
                Selector::from_iter(Some(Expression {
                    key: "foo".into(),
                    operator: Operator::In,
                    values: Some(Some("bar".to_string()).into_iter().collect()),
                })),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "baz")]),
                true,
                "In expression match",
            ),
            (
                Selector::from_iter(Some(Expression {
                    key: "foo".into(),
                    operator: Operator::NotIn,
                    values: Some(Some("quux".to_string()).into_iter().collect()),
                })),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "baz")]),
                true,
                "NotIn expression match",
            ),
            (
                Selector::from_iter(Some(Expression {
                    key: "foo".into(),
                    operator: Operator::NotIn,
                    values: Some(Some("bar".to_string()).into_iter().collect()),
                })),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "baz")]),
                false,
                "NotIn expression non-match",
            ),
            (
                Selector::new(Map::from([("foo".to_string(), "bar".to_string())]), vec![
                    Expression {
                        key: "bah".into(),
                        operator: Operator::In,
                        values: Some(Some("bar".to_string()).into_iter().collect()),
                    },
                ]),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "baz")]),
                false,
                "matches labels but not expressions",
            ),
            (
                Selector::new(Map::from([("foo".to_string(), "bar".to_string())]), vec![
                    Expression {
                        key: "bah".into(),
                        operator: Operator::In,
                        values: Some(Some("bar".to_string()).into_iter().collect()),
                    },
                ]),
                Labels::from_iter(vec![("foo", "bar"), ("bah", "bar")]),
                true,
                "matches both labels and expressions",
            ),
        ] {
            assert_eq!(selector.matches(labels), *matches, "{}", msg);
        }
    }
}
