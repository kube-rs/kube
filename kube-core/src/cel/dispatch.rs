//! Runtime type dispatch for CEL functions with name collisions.
//!
//! The `cel` crate registers functions by name only (no typed overloads).
//! When the same function name applies to multiple types (e.g., `indexOf` for
//! both strings and lists), this module provides unified dispatch functions
//! that route to the correct implementation based on the runtime type of `this`.

use std::sync::Arc;

use cel::{
    Context, ExecutionError, ResolveResult,
    extractors::{Arguments, This},
    objects::Value,
};

/// Register dispatch functions for names shared across multiple types or
/// that override cel built-in functions.
pub fn register(ctx: &mut Context<'_>) {
    ctx.add_function("indexOf", index_of);
    ctx.add_function("lastIndexOf", last_index_of);
    ctx.add_function("string", string_dispatch);
    ctx.add_function("reverse", reverse);
    ctx.add_function("min", min_dispatch);
    ctx.add_function("max", max_dispatch);
}

// ---------------------------------------------------------------------------
// indexOf / lastIndexOf
// ---------------------------------------------------------------------------

#[allow(unused_variables)]
fn index_of(This(this): This<Value>, Arguments(args): Arguments) -> ResolveResult {
    match this {
        Value::String(s) => super::strings::string_index_of(This(s), Arguments(args)),
        Value::List(list) => super::lists::list_index_of(&list, &args),
        _ => Err(ExecutionError::function_error(
            "indexOf",
            format!("indexOf not supported on type {:?}", this.type_of()),
        )),
    }
}

#[allow(unused_variables)]
fn last_index_of(This(this): This<Value>, Arguments(args): Arguments) -> ResolveResult {
    match this {
        Value::String(s) => super::strings::string_last_index_of(This(s), Arguments(args)),
        Value::List(list) => super::lists::list_last_index_of(&list, &args),
        _ => Err(ExecutionError::function_error(
            "lastIndexOf",
            format!("lastIndexOf not supported on type {:?}", this.type_of()),
        )),
    }
}

// ---------------------------------------------------------------------------
// string (cel built-in override)
// ---------------------------------------------------------------------------
//
// The standard conversions below mirror `cel::functions::string` (cel 0.12).

fn string_dispatch(This(this): This<Value>) -> ResolveResult {
    builtin_string_fallback(this)
}

/// Reimplements cel's built-in `string()` for standard types.
fn builtin_string_fallback(this: Value) -> ResolveResult {
    match this {
        Value::String(_) => Ok(this),
        Value::Int(n) => Ok(Value::String(Arc::new(n.to_string()))),
        Value::UInt(n) => Ok(Value::String(Arc::new(n.to_string()))),
        Value::Float(f) => Ok(Value::String(Arc::new(f.to_string()))),
        Value::Bytes(ref b) => Ok(Value::String(Arc::new(
            String::from_utf8_lossy(b.as_slice()).into(),
        ))),
        Value::Timestamp(ref t) => Ok(Value::String(Arc::new(t.to_rfc3339()))),
        Value::Duration(ref d) => Ok(Value::String(Arc::new(format_cel_duration(
            d.num_nanoseconds().unwrap_or(d.num_seconds() * 1_000_000_000),
        )))),
        _ => Err(ExecutionError::function_error(
            "string",
            format!("cannot convert {:?} to string", this.type_of()),
        )),
    }
}

/// Format nanoseconds matching Go's `time.Duration.String()`.
fn format_cel_duration(total_nanos: i64) -> String {
    if total_nanos == 0 {
        return "0s".into();
    }

    let neg = total_nanos < 0;
    let u = total_nanos.unsigned_abs();
    let mut result = String::new();
    if neg {
        result.push('-');
    }

    const NS_SECOND: u64 = 1_000_000_000;
    const NS_MINUTE: u64 = 60 * NS_SECOND;
    const NS_HOUR: u64 = 60 * NS_MINUTE;

    if u >= NS_SECOND {
        let hours = u / NS_HOUR;
        let mins = (u % NS_HOUR) / NS_MINUTE;
        let secs = (u % NS_MINUTE) / NS_SECOND;
        let frac = u % NS_SECOND;
        if hours > 0 {
            result.push_str(&format!("{hours}h"));
        }
        if hours > 0 || mins > 0 {
            result.push_str(&format!("{mins}m"));
        }
        if frac > 0 {
            let frac_s = format!("{frac:09}");
            let frac_s = frac_s.trim_end_matches('0');
            result.push_str(&format!("{secs}.{frac_s}s"));
        } else {
            result.push_str(&format!("{secs}s"));
        }
    } else {
        const NS_MILLISECOND: u64 = 1_000_000;
        const NS_MICROSECOND: u64 = 1_000;
        if u >= NS_MILLISECOND {
            let ms = u as f64 / NS_MILLISECOND as f64;
            let s = format!("{ms:.3}");
            result.push_str(s.trim_end_matches('0').trim_end_matches('.'));
            result.push_str("ms");
        } else if u >= NS_MICROSECOND {
            let us = u as f64 / NS_MICROSECOND as f64;
            let s = format!("{us:.3}");
            result.push_str(s.trim_end_matches('0').trim_end_matches('.'));
            result.push_str("µs");
        } else {
            result.push_str(&format!("{u}ns"));
        }
    }
    result
}

// ---------------------------------------------------------------------------
// reverse (string → reversed string, list → reversed list)
// ---------------------------------------------------------------------------

#[allow(unused_variables)]
fn reverse(This(this): This<Value>) -> ResolveResult {
    match this {
        Value::String(s) => super::strings::string_reverse(This(s)),
        Value::List(list) => super::lists::list_reverse_value(This(list)),
        _ => Err(ExecutionError::function_error(
            "reverse",
            format!("reverse not supported on type {:?}", this.type_of()),
        )),
    }
}

// ---------------------------------------------------------------------------
// min / max (list method vs cel built-in variadic)
// ---------------------------------------------------------------------------

fn min_dispatch(This(this): This<Value>, Arguments(args): Arguments) -> ResolveResult {
    match this {
        Value::List(list) if args.is_empty() => super::lists::list_min(This(list)),
        _ => {
            let mut all_args = vec![this];
            all_args.extend(args.iter().cloned());
            cel::functions::min(Arguments(Arc::new(all_args)))
        }
    }
}

fn max_dispatch(This(this): This<Value>, Arguments(args): Arguments) -> ResolveResult {
    match this {
        Value::List(list) if args.is_empty() => super::lists::list_max(This(list)),
        _ => {
            let mut all_args = vec![this];
            all_args.extend(args.iter().cloned());
            cel::functions::max(Arguments(Arc::new(all_args)))
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(dead_code)]
    use cel::{Context, Program, Value};

    fn eval(expr: &str) -> Value {
        let mut ctx = Context::default();
        crate::cel::register_all(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap()
    }

    fn eval_err(expr: &str) -> cel::ExecutionError {
        let mut ctx = Context::default();
        crate::cel::register_all(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap_err()
    }

    #[test]
    fn test_index_of_unsupported_type() {
        eval_err("true.indexOf('x')");
    }

    #[test]
    fn test_last_index_of_unsupported_type() {
        eval_err("true.lastIndexOf('x')");
    }

    #[test]
    fn test_string_int() {
        assert_eq!(eval("42.string()"), Value::String("42".to_string().into()));
    }

    #[test]
    fn test_string_uint() {
        assert_eq!(eval("42u.string()"), Value::String("42".to_string().into()));
    }

    #[test]
    fn test_string_float() {
        assert_eq!(eval("3.14.string()"), Value::String("3.14".to_string().into()));
    }

    #[test]
    fn test_string_string() {
        assert_eq!(
            eval("'hello'.string()"),
            Value::String("hello".to_string().into())
        );
    }

    #[test]
    fn test_string_bytes() {
        assert_eq!(eval("b'abc'.string()"), Value::String("abc".to_string().into()));
    }

    #[test]
    fn test_string_unsupported_type() {
        eval_err("true.string()");
    }

    #[test]
    fn test_min_list_method() {
        assert_eq!(eval("[3, 1, 2].min()"), Value::Int(1));
    }

    #[test]
    fn test_max_list_method() {
        assert_eq!(eval("[3, 1, 2].max()"), Value::Int(3));
    }

    #[test]
    fn test_min_global_variadic() {
        assert_eq!(eval("min(5, 3)"), Value::Int(3));
    }

    #[test]
    fn test_max_global_variadic() {
        assert_eq!(eval("max(5, 3)"), Value::Int(5));
    }
}
