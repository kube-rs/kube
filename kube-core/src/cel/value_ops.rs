//! Shared value comparison and arithmetic helpers.
//!
//! Used by `lists` and `sets` modules to avoid code duplication.

#![allow(dead_code)]

use cel::{ExecutionError, objects::Value};
use std::cmp::Ordering;

pub(crate) fn compare_values(a: &Value, b: &Value) -> Result<Ordering, ExecutionError> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
        (Value::UInt(a), Value::UInt(b)) => Ok(a.cmp(b)),
        (Value::Float(a), Value::Float(b)) => Ok(a.partial_cmp(b).unwrap_or(Ordering::Equal)),
        (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
        (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
        _ => Err(ExecutionError::function_error(
            "compare",
            "cannot compare values of different types",
        )),
    }
}

pub(crate) fn val_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::UInt(a), Value::UInt(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        _ => false,
    }
}

pub(crate) fn val_lt(a: &Value, b: &Value) -> Result<bool, ExecutionError> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(a < b),
        (Value::UInt(a), Value::UInt(b)) => Ok(a < b),
        (Value::Float(a), Value::Float(b)) => Ok(a < b),
        (Value::String(a), Value::String(b)) => Ok(a < b),
        (Value::Bool(a), Value::Bool(b)) => Ok(!a & b),
        _ => Err(ExecutionError::function_error(
            "compare",
            "cannot compare values of different types",
        )),
    }
}

pub(crate) fn val_le(a: &Value, b: &Value) -> Result<bool, ExecutionError> {
    Ok(val_eq(a, b) || val_lt(a, b)?)
}

pub(crate) fn val_add(a: &Value, b: &Value) -> Result<Value, ExecutionError> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::UInt(a), Value::UInt(b)) => Ok(Value::UInt(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        _ => Err(ExecutionError::function_error(
            "sum",
            "cannot sum values of this type",
        )),
    }
}
