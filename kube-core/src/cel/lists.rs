//! Kubernetes CEL list extension functions.
//!
//! Provides list functions available in Kubernetes CEL expressions,
//! matching the behavior of `k8s.io/apiserver/pkg/cel/library/lists.go`.

use cel::{
    Context, ExecutionError, ResolveResult,
    extractors::{Arguments, This},
    objects::{OptionalValue, Value},
};
use std::{cmp::Ordering, sync::Arc};

use super::value_ops::{compare_values, val_add, val_eq, val_le, val_lt};

/// Register all list extension functions.
pub fn register(ctx: &mut Context<'_>) {
    ctx.add_function("isSorted", is_sorted);
    ctx.add_function("sum", sum);
    // min/max are registered via dispatch module to handle
    // name collision with cel built-in variadic min/max.
    // indexOf/lastIndexOf are registered via dispatch module to handle
    // name collision between string and list versions.
    ctx.add_function("slice", slice);
    ctx.add_function("sort", sort);
    ctx.add_function("flatten", flatten);
    // reverse is registered via dispatch module to handle
    // name collision between string and list versions.
    ctx.add_function("distinct", distinct);
    ctx.add_function("first", list_first);
    ctx.add_function("last", list_last);
    ctx.add_function("lists.range", lists_range);
}

/// `<list>.isSorted() -> bool`
///
/// Returns true if the list elements are in sorted (ascending) order.
fn is_sorted(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    for window in this.windows(2) {
        if !val_le(&window[0], &window[1])? {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// `<list>.sum() -> T`
///
/// Returns the sum of all elements. Empty list returns 0 for int, 0u for uint, 0.0 for double.
fn sum(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    if this.is_empty() {
        return Ok(Value::Int(0));
    }

    let mut acc = this[0].clone();
    for item in this.iter().skip(1) {
        acc = val_add(&acc, item)?;
    }
    Ok(acc)
}

/// `<list>.min() -> T`
///
/// Returns the minimum element. Errors on empty list.
/// Called from dispatch module for list/variadic dispatch.
pub(crate) fn list_min(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    if this.is_empty() {
        return Err(cel::ExecutionError::function_error(
            "min",
            "cannot call min on empty list",
        ));
    }
    let mut result = this[0].clone();
    for item in this.iter().skip(1) {
        if val_lt(item, &result)? {
            result = item.clone();
        }
    }
    Ok(result)
}

/// `<list>.max() -> T`
///
/// Returns the maximum element. Errors on empty list.
/// Called from dispatch module for list/variadic dispatch.
pub(crate) fn list_max(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    if this.is_empty() {
        return Err(cel::ExecutionError::function_error(
            "max",
            "cannot call max on empty list",
        ));
    }
    let mut result = this[0].clone();
    for item in this.iter().skip(1) {
        if val_lt(&result, item)? {
            result = item.clone();
        }
    }
    Ok(result)
}

/// `<list>.indexOf(T) -> int`
///
/// Returns the index of the first occurrence of the value, or -1 if not found.
pub(crate) fn list_index_of(list: &[Value], args: &[Value]) -> ResolveResult {
    let target = args
        .first()
        .ok_or_else(|| ExecutionError::function_error("indexOf", "expected argument"))?;
    for (i, item) in list.iter().enumerate() {
        if val_eq(item, target) {
            return Ok(Value::Int(i as i64));
        }
    }
    Ok(Value::Int(-1))
}

/// `<list>.lastIndexOf(T) -> int`
///
/// Returns the index of the last occurrence of the value, or -1 if not found.
pub(crate) fn list_last_index_of(list: &[Value], args: &[Value]) -> ResolveResult {
    let target = args
        .first()
        .ok_or_else(|| ExecutionError::function_error("lastIndexOf", "expected argument"))?;
    let mut result: i64 = -1;
    for (i, item) in list.iter().enumerate() {
        if val_eq(item, target) {
            result = i as i64;
        }
    }
    Ok(Value::Int(result))
}

/// `<list>.slice(int, int) -> list`
///
/// Returns a sub-list from start (inclusive) to end (exclusive).
fn slice(This(this): This<Arc<Vec<Value>>>, start: i64, end: i64) -> ResolveResult {
    let len = this.len() as i64;
    if start < 0 || start > len || end < start || end > len {
        return Err(cel::ExecutionError::function_error(
            "slice",
            format!("slice({start}, {end}) out of range for list of length {len}"),
        ));
    }
    let result: Vec<Value> = this[start as usize..end as usize].to_vec();
    Ok(Value::List(Arc::new(result)))
}

/// `<list>.sort() -> list`
///
/// Returns a new list with elements in sorted (ascending) order.
fn sort(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    let mut items: Vec<Value> = this.iter().cloned().collect();
    let mut err: Option<ExecutionError> = None;
    items.sort_by(|a, b| match compare_values(a, b) {
        Ok(ord) => ord,
        Err(e) => {
            err = Some(e);
            Ordering::Equal
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(Value::List(Arc::new(items)))
}

/// `<list>.flatten() -> list`
/// `<list>.flatten(<int>) -> list`
///
/// Flattens a list. Without arguments, flattens one level.
/// With a depth argument, flattens up to that many levels.
fn flatten(This(this): This<Arc<Vec<Value>>>, Arguments(args): Arguments) -> ResolveResult {
    let depth = match args.first() {
        Some(Value::Int(d)) => {
            if *d < 0 {
                return Err(ExecutionError::function_error(
                    "flatten",
                    "depth must be non-negative",
                ));
            }
            *d as usize
        }
        None => 1,
        _ => {
            return Err(ExecutionError::function_error(
                "flatten",
                "expected int argument for depth",
            ));
        }
    };
    Ok(Value::List(Arc::new(flatten_recursive(&this, depth))))
}

fn flatten_recursive(items: &[Value], depth: usize) -> Vec<Value> {
    if depth == 0 {
        return items.to_vec();
    }
    let mut result = Vec::new();
    for item in items {
        match item {
            Value::List(inner) => {
                result.extend(flatten_recursive(inner, depth - 1));
            }
            other => result.push(other.clone()),
        }
    }
    result
}

/// `lists.range(n) -> list`
///
/// Returns a list of integers from 0 to n-1.
fn lists_range(n: i64) -> ResolveResult {
    if n < 0 {
        return Err(ExecutionError::function_error(
            "lists.range",
            "range argument must be non-negative",
        ));
    }
    let items: Vec<Value> = (0..n).map(Value::Int).collect();
    Ok(Value::List(Arc::new(items)))
}

/// `<list>.reverse() -> list`
///
/// Returns a new list with elements in reverse order.
/// Called from dispatch module for string/list dispatch.
pub(crate) fn list_reverse_value(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    let mut result: Vec<Value> = this.iter().cloned().collect();
    result.reverse();
    Ok(Value::List(Arc::new(result)))
}

/// `<list>.first() -> optional<T>`
///
/// Returns `optional.of(first_element)` if non-empty, `optional.none()` if empty.
fn list_first(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    match this.first() {
        Some(v) => Ok(Value::Opaque(Arc::new(OptionalValue::of(v.clone())))),
        None => Ok(Value::Opaque(Arc::new(OptionalValue::none()))),
    }
}

/// `<list>.last() -> optional<T>`
///
/// Returns `optional.of(last_element)` if non-empty, `optional.none()` if empty.
fn list_last(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    match this.last() {
        Some(v) => Ok(Value::Opaque(Arc::new(OptionalValue::of(v.clone())))),
        None => Ok(Value::Opaque(Arc::new(OptionalValue::none()))),
    }
}

/// `<list>.distinct() -> list`
///
/// Returns a new list with duplicate elements removed, preserving order.
fn distinct(This(this): This<Arc<Vec<Value>>>) -> ResolveResult {
    let mut result = Vec::new();
    for item in this.iter() {
        if !result.iter().any(|s| val_eq(s, item)) {
            result.push(item.clone());
        }
    }
    Ok(Value::List(Arc::new(result)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cel::Program;

    fn eval(expr: &str) -> Value {
        let mut ctx = Context::default();
        register(&mut ctx);
        crate::cel::dispatch::register(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap()
    }

    #[test]
    fn test_is_sorted() {
        assert_eq!(eval("[1, 2, 3].isSorted()"), Value::Bool(true));
        assert_eq!(eval("[3, 1, 2].isSorted()"), Value::Bool(false));
        assert_eq!(eval("[].isSorted()"), Value::Bool(true));
        assert_eq!(eval("['a', 'b', 'c'].isSorted()"), Value::Bool(true));
    }

    #[test]
    fn test_sum() {
        assert_eq!(eval("[1, 2, 3].sum()"), Value::Int(6));
        assert_eq!(eval("[1.5, 2.5].sum()"), Value::Float(4.0));
        assert_eq!(eval("[].sum()"), Value::Int(0));
    }

    #[test]
    fn test_min_max() {
        assert_eq!(eval("[3, 1, 2].min()"), Value::Int(1));
        assert_eq!(eval("[3, 1, 2].max()"), Value::Int(3));
    }

    #[test]
    fn test_index_of() {
        assert_eq!(eval("[1, 2, 3, 2].indexOf(2)"), Value::Int(1));
        assert_eq!(eval("[1, 2, 3].indexOf(4)"), Value::Int(-1));
    }

    #[test]
    fn test_last_index_of() {
        assert_eq!(eval("[1, 2, 3, 2].lastIndexOf(2)"), Value::Int(3));
    }

    #[test]
    fn test_slice() {
        assert_eq!(
            eval("[1, 2, 3, 4].slice(1, 3)"),
            Value::List(Arc::new(vec![Value::Int(2), Value::Int(3)]))
        );
    }

    #[test]
    fn test_flatten() {
        assert_eq!(
            eval("[[1, 2], [3, 4]].flatten()"),
            Value::List(Arc::new(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
            ]))
        );
    }

    #[test]
    fn test_reverse() {
        assert_eq!(
            eval("[1, 2, 3].reverse()"),
            Value::List(Arc::new(vec![Value::Int(3), Value::Int(2), Value::Int(1)]))
        );
    }

    #[test]
    fn test_distinct() {
        assert_eq!(
            eval("[1, 2, 2, 3, 1].distinct()"),
            Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))
        );
    }

    // --- first / last ---

    #[test]
    fn test_first() {
        assert_eq!(eval("[1, 2, 3].first().hasValue()"), Value::Bool(true));
        assert_eq!(eval("[1, 2, 3].first().value()"), Value::Int(1));
    }

    #[test]
    fn test_first_empty() {
        assert_eq!(eval("[].first().hasValue()"), Value::Bool(false));
    }

    #[test]
    fn test_last() {
        assert_eq!(eval("[1, 2, 3].last().hasValue()"), Value::Bool(true));
        assert_eq!(eval("[1, 2, 3].last().value()"), Value::Int(3));
    }

    #[test]
    fn test_last_empty() {
        assert_eq!(eval("[].last().hasValue()"), Value::Bool(false));
    }

    // --- Error & edge case tests ---

    fn eval_err(expr: &str) -> cel::ExecutionError {
        let mut ctx = Context::default();
        register(&mut ctx);
        crate::cel::dispatch::register(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap_err()
    }

    #[test]
    fn test_min_empty_list() {
        eval_err("[].min()");
    }

    #[test]
    fn test_max_empty_list() {
        eval_err("[].max()");
    }

    #[test]
    fn test_min_max_single_element() {
        assert_eq!(eval("[42].min()"), Value::Int(42));
        assert_eq!(eval("[42].max()"), Value::Int(42));
    }

    #[test]
    fn test_min_max_strings() {
        assert_eq!(eval("['c', 'a', 'b'].min()"), Value::String(Arc::new("a".into())));
        assert_eq!(eval("['c', 'a', 'b'].max()"), Value::String(Arc::new("c".into())));
    }

    #[test]
    fn test_slice_errors() {
        eval_err("[1, 2, 3].slice(-1, 2)"); // start < 0
        eval_err("[1, 2, 3].slice(2, 1)"); // end < start
        eval_err("[1, 2, 3].slice(0, 5)"); // end > len
        eval_err("[1, 2, 3].slice(5, 5)"); // start > len
    }

    #[test]
    fn test_slice_empty_range() {
        assert_eq!(eval("[1, 2, 3].slice(2, 2)"), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_flatten_mixed() {
        // Non-list items are kept as-is
        assert_eq!(
            eval("[1, [2, 3], 4].flatten()"),
            Value::List(Arc::new(vec![
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
            ]))
        );
    }

    #[test]
    fn test_flatten_one_level_only() {
        // Only flattens one level deep
        assert_eq!(
            eval("[[1, [2, 3]]].flatten()"),
            Value::List(Arc::new(vec![
                Value::Int(1),
                Value::List(Arc::new(vec![Value::Int(2), Value::Int(3)])),
            ]))
        );
    }

    #[test]
    fn test_is_sorted_with_equal_elements() {
        assert_eq!(eval("[1, 1, 2].isSorted()"), Value::Bool(true));
    }

    #[test]
    fn test_distinct_strings() {
        assert_eq!(
            eval("['a', 'b', 'a'].distinct()"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("a".into())),
                Value::String(Arc::new("b".into())),
            ]))
        );
    }

    #[test]
    fn test_reverse_empty() {
        assert_eq!(eval("[].reverse()"), Value::List(Arc::new(vec![])));
    }

    // --- sort tests ---

    #[test]
    fn test_sort_ints() {
        assert_eq!(
            eval("[3, 1, 2].sort()"),
            Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))
        );
    }

    #[test]
    fn test_sort_strings() {
        assert_eq!(
            eval("['c', 'a', 'b'].sort()"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("a".into())),
                Value::String(Arc::new("b".into())),
                Value::String(Arc::new("c".into())),
            ]))
        );
    }

    #[test]
    fn test_sort_empty() {
        assert_eq!(eval("[].sort()"), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_sort_already_sorted() {
        assert_eq!(
            eval("[1, 2, 3].sort()"),
            Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3)]))
        );
    }

    // --- lists.range tests ---

    #[test]
    fn test_lists_range() {
        assert_eq!(
            eval("lists.range(5)"),
            Value::List(Arc::new(vec![
                Value::Int(0),
                Value::Int(1),
                Value::Int(2),
                Value::Int(3),
                Value::Int(4),
            ]))
        );
    }

    #[test]
    fn test_lists_range_zero() {
        assert_eq!(eval("lists.range(0)"), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_lists_range_negative() {
        eval_err("lists.range(-1)");
    }

    // --- flatten with depth tests ---

    #[test]
    fn test_flatten_depth_two() {
        assert_eq!(
            eval("[[1, [2]], [3]].flatten(2)"),
            Value::List(Arc::new(vec![Value::Int(1), Value::Int(2), Value::Int(3),]))
        );
    }

    #[test]
    fn test_flatten_depth_zero() {
        // depth 0 = no flattening
        assert_eq!(
            eval("[[1, 2]].flatten(0)"),
            Value::List(Arc::new(vec![Value::List(Arc::new(vec![
                Value::Int(1),
                Value::Int(2),
            ]))]))
        );
    }

    // --- cel-go parity tests ---

    #[test]
    fn test_reverse_strings() {
        assert_eq!(
            eval("['are', 'you', 'as', 'bored', 'as', 'I', 'am'].reverse()"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("am".into())),
                Value::String(Arc::new("I".into())),
                Value::String(Arc::new("as".into())),
                Value::String(Arc::new("bored".into())),
                Value::String(Arc::new("as".into())),
                Value::String(Arc::new("you".into())),
                Value::String(Arc::new("are".into())),
            ]))
        );
    }

    #[test]
    fn test_slice_at_end() {
        assert_eq!(eval("[1, 2, 3, 4].slice(4, 4)"), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_slice_negative_error() {
        eval_err("[1, 2, 3, 4].slice(-5, 10)");
    }

    #[test]
    fn test_flatten_negative_depth_error() {
        eval_err("[].flatten(-1)");
    }

    #[test]
    fn test_first_single() {
        assert_eq!(eval("[42].first().value()"), Value::Int(42));
    }

    #[test]
    fn test_last_single() {
        assert_eq!(eval("[42].last().value()"), Value::Int(42));
    }

    #[test]
    fn test_first_strings() {
        assert_eq!(
            eval("['a', 'b', 'c'].first().value()"),
            Value::String(Arc::new("a".into()))
        );
    }

    #[test]
    fn test_last_strings() {
        assert_eq!(
            eval("['a', 'b', 'c'].last().value()"),
            Value::String(Arc::new("c".into()))
        );
    }
}
