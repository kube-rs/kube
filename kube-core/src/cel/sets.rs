//! Kubernetes CEL sets extension functions.
//!
//! Provides set operations on lists, matching `cel-go/ext/sets.go`.
//! These are namespaced functions called as `sets.contains(a, b)`.

use cel::{Context, ResolveResult, objects::Value};
use std::sync::Arc;

use super::value_ops::val_eq;

/// Register all set extension functions.
pub fn register(ctx: &mut Context<'_>) {
    ctx.add_function("sets.contains", sets_contains);
    ctx.add_function("sets.equivalent", sets_equivalent);
    ctx.add_function("sets.intersects", sets_intersects);
}

/// `sets.contains(list, list) -> bool`
///
/// Returns true if the first list contains all elements of the second list.
fn sets_contains(a: Arc<Vec<Value>>, b: Arc<Vec<Value>>) -> ResolveResult {
    for item in b.iter() {
        if !a.iter().any(|x| val_eq(x, item)) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// `sets.equivalent(list, list) -> bool`
///
/// Returns true if both lists contain the same set of elements
/// (ignoring duplicates and order).
fn sets_equivalent(a: Arc<Vec<Value>>, b: Arc<Vec<Value>>) -> ResolveResult {
    // a contains all of b AND b contains all of a
    for item in b.iter() {
        if !a.iter().any(|x| val_eq(x, item)) {
            return Ok(Value::Bool(false));
        }
    }
    for item in a.iter() {
        if !b.iter().any(|x| val_eq(x, item)) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// `sets.intersects(list, list) -> bool`
///
/// Returns true if the two lists share at least one common element.
fn sets_intersects(a: Arc<Vec<Value>>, b: Arc<Vec<Value>>) -> ResolveResult {
    for item in a.iter() {
        if b.iter().any(|x| val_eq(x, item)) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cel::Program;

    fn eval(expr: &str) -> Value {
        let mut ctx = Context::default();
        register(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap()
    }

    #[test]
    fn test_contains() {
        assert_eq!(eval("sets.contains([1, 2, 3], [1, 2])"), Value::Bool(true));
        assert_eq!(eval("sets.contains([1, 2, 3], [4])"), Value::Bool(false));
        assert_eq!(eval("sets.contains([1, 2, 3], [])"), Value::Bool(true));
    }

    #[test]
    fn test_equivalent() {
        assert_eq!(eval("sets.equivalent([1, 2, 3], [3, 2, 1])"), Value::Bool(true));
        assert_eq!(eval("sets.equivalent([1, 2, 2], [1, 2])"), Value::Bool(true));
        assert_eq!(eval("sets.equivalent([1, 2], [1, 3])"), Value::Bool(false));
    }

    #[test]
    fn test_intersects() {
        assert_eq!(eval("sets.intersects([1, 2], [2, 3])"), Value::Bool(true));
        assert_eq!(eval("sets.intersects([1, 2], [3, 4])"), Value::Bool(false));
        assert_eq!(eval("sets.intersects([], [1])"), Value::Bool(false));
    }

    // --- Edge case tests ---

    #[test]
    fn test_equivalent_empty() {
        assert_eq!(eval("sets.equivalent([], [])"), Value::Bool(true));
    }

    #[test]
    fn test_intersects_both_empty() {
        assert_eq!(eval("sets.intersects([], [])"), Value::Bool(false));
    }

    #[test]
    fn test_contains_strings() {
        assert_eq!(
            eval("sets.contains(['a', 'b', 'c'], ['a', 'c'])"),
            Value::Bool(true)
        );
        assert_eq!(eval("sets.contains(['a', 'b'], ['d'])"), Value::Bool(false));
    }

    #[test]
    fn test_intersects_strings() {
        assert_eq!(eval("sets.intersects(['a', 'b'], ['b', 'c'])"), Value::Bool(true));
    }

    // --- cel-go parity tests ---

    #[test]
    fn test_contains_negated() {
        assert_eq!(eval("!sets.contains([1], [2])"), Value::Bool(true));
    }

    #[test]
    fn test_equivalent_negated() {
        assert_eq!(eval("!sets.equivalent([2, 1], [1])"), Value::Bool(true));
    }

    #[test]
    fn test_intersects_negated() {
        assert_eq!(eval("!sets.intersects([], [])"), Value::Bool(true));
        assert_eq!(eval("!sets.intersects([1], [2])"), Value::Bool(true));
    }

    #[test]
    fn test_contains_empty_superset() {
        assert_eq!(eval("sets.contains([], [])"), Value::Bool(true));
    }

    #[test]
    fn test_contains_with_duplicates() {
        assert_eq!(eval("sets.contains([1, 1, 2, 2, 3], [1, 2])"), Value::Bool(true));
    }

    #[test]
    fn test_equivalent_with_duplicates() {
        assert_eq!(eval("sets.equivalent([1, 1, 2], [2, 2, 1])"), Value::Bool(true));
    }
}
