//! Kubernetes CEL string extension functions.
//!
//! Provides the string functions available in Kubernetes CEL expressions,
//! matching the behavior of `cel-go/ext/strings.go`.

use cel::{
    Context, ExecutionError, ResolveResult,
    extractors::{Arguments, This},
    objects::Value,
};
use std::sync::Arc;

/// Register all string extension functions.
pub fn register(ctx: &mut Context<'_>) {
    ctx.add_function("charAt", char_at);
    // indexOf/lastIndexOf are registered in lists.rs with runtime type dispatch
    // to avoid name collisions between string and list versions.
    ctx.add_function("lowerAscii", lower_ascii);
    ctx.add_function("upperAscii", upper_ascii);
    ctx.add_function("replace", string_replace);
    ctx.add_function("split", string_split);
    ctx.add_function("substring", substring);
    ctx.add_function("trim", trim);
    ctx.add_function("join", join);
    ctx.add_function("strings.quote", strings_quote);
}

/// `<string>.charAt(<int>) -> <string>`
///
/// Returns the character at the given index as a single-character string.
/// If `idx == len`, returns `""` (matching cel-go behavior).
fn char_at(This(this): This<Arc<String>>, idx: i64) -> ResolveResult {
    let chars: Vec<char> = this.chars().collect();
    if idx < 0 || idx as usize > chars.len() {
        return Err(ExecutionError::function_error(
            "charAt",
            format!("index {idx} out of range for string of length {}", chars.len()),
        ));
    }
    if idx as usize == chars.len() {
        return Ok(Value::String(Arc::new(String::new())));
    }
    Ok(Value::String(Arc::new(chars[idx as usize].to_string())))
}

/// `<string>.indexOf(<string>) -> <int>`
/// `<string>.indexOf(<string>, <int>) -> <int>`
pub(crate) fn string_index_of(This(this): This<Arc<String>>, Arguments(args): Arguments) -> ResolveResult {
    let search = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(ExecutionError::function_error(
                "indexOf",
                "expected string argument",
            ));
        }
    };
    let offset: usize = match args.get(1) {
        Some(Value::Int(n)) => (*n).max(0) as usize,
        _ => 0,
    };

    let chars: Vec<char> = this.chars().collect();
    let search_chars: Vec<char> = search.chars().collect();

    if search_chars.is_empty() {
        return Ok(Value::Int(offset as i64));
    }

    for i in offset..chars.len() {
        if i + search_chars.len() <= chars.len() && chars[i..i + search_chars.len()] == search_chars[..] {
            return Ok(Value::Int(i as i64));
        }
    }
    Ok(Value::Int(-1))
}

/// `<string>.lastIndexOf(<string>) -> <int>`
/// `<string>.lastIndexOf(<string>, <int>) -> <int>`
pub(crate) fn string_last_index_of(
    This(this): This<Arc<String>>,
    Arguments(args): Arguments,
) -> ResolveResult {
    let search = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(ExecutionError::function_error(
                "lastIndexOf",
                "expected string argument",
            ));
        }
    };

    let chars: Vec<char> = this.chars().collect();
    let search_chars: Vec<char> = search.chars().collect();

    let end: usize = match args.get(1) {
        Some(Value::Int(n)) => ((*n).max(0) as usize).min(chars.len()),
        _ => chars.len(),
    };

    if search_chars.is_empty() {
        return Ok(Value::Int(end as i64));
    }

    let mut result: i64 = -1;
    for i in 0..end {
        if i + search_chars.len() <= end && chars[i..i + search_chars.len()] == search_chars[..] {
            result = i as i64;
        }
    }
    Ok(Value::Int(result))
}

/// `<string>.lowerAscii() -> <string>`
fn lower_ascii(This(this): This<Arc<String>>) -> ResolveResult {
    Ok(Value::String(Arc::new(this.to_ascii_lowercase())))
}

/// `<string>.upperAscii() -> <string>`
fn upper_ascii(This(this): This<Arc<String>>) -> ResolveResult {
    Ok(Value::String(Arc::new(this.to_ascii_uppercase())))
}

/// `<string>.replace(<string>, <string>) -> <string>`
/// `<string>.replace(<string>, <string>, <int>) -> <string>`
fn string_replace(This(this): This<Arc<String>>, Arguments(args): Arguments) -> ResolveResult {
    let from = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(ExecutionError::function_error(
                "replace",
                "expected string argument",
            ));
        }
    };
    let to = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(ExecutionError::function_error(
                "replace",
                "expected string argument",
            ));
        }
    };

    let result = match args.get(2) {
        Some(Value::Int(n)) => this.replacen(from.as_str(), to.as_str(), (*n).max(0) as usize),
        _ => this.replace(from.as_str(), to.as_str()),
    };
    Ok(Value::String(Arc::new(result)))
}

/// `<string>.split(<string>) -> <list<string>>`
/// `<string>.split(<string>, <int>) -> <list<string>>`
fn string_split(This(this): This<Arc<String>>, Arguments(args): Arguments) -> ResolveResult {
    let separator = match args.first() {
        Some(Value::String(s)) => s.clone(),
        _ => {
            return Err(ExecutionError::function_error(
                "split",
                "expected string argument",
            ));
        }
    };

    let parts: Vec<Value> = match args.get(1) {
        Some(Value::Int(n)) if *n == 0 => vec![],
        Some(Value::Int(n)) if *n < 0 => this
            .split(separator.as_str())
            .map(|s| Value::String(Arc::new(s.to_string())))
            .collect(),
        Some(Value::Int(n)) => this
            .splitn(*n as usize, separator.as_str())
            .map(|s| Value::String(Arc::new(s.to_string())))
            .collect(),
        _ => this
            .split(separator.as_str())
            .map(|s| Value::String(Arc::new(s.to_string())))
            .collect(),
    };
    Ok(Value::List(Arc::new(parts)))
}

/// `<string>.substring(<int>) -> <string>`
/// `<string>.substring(<int>, <int>) -> <string>`
fn substring(This(this): This<Arc<String>>, Arguments(args): Arguments) -> ResolveResult {
    let start = match args.first() {
        Some(Value::Int(n)) => *n,
        _ => {
            return Err(ExecutionError::function_error(
                "substring",
                "expected int argument",
            ));
        }
    };

    let chars: Vec<char> = this.chars().collect();
    let len = chars.len();

    if start < 0 || start as usize > len {
        return Err(ExecutionError::function_error(
            "substring",
            format!("start index {start} out of range for string of length {len}"),
        ));
    }

    let end = match args.get(1) {
        Some(Value::Int(n)) => {
            if *n < start || *n as usize > len {
                return Err(ExecutionError::function_error(
                    "substring",
                    format!("end index {n} out of range"),
                ));
            }
            *n as usize
        }
        _ => len,
    };

    let result: String = chars[start as usize..end].iter().collect();
    Ok(Value::String(Arc::new(result)))
}

/// `<string>.trim() -> <string>`
fn trim(This(this): This<Arc<String>>) -> ResolveResult {
    Ok(Value::String(Arc::new(this.trim().to_string())))
}

/// `<list<string>>.join() -> <string>`
/// `<list<string>>.join(<string>) -> <string>`
fn join(This(this): This<Arc<Vec<Value>>>, Arguments(args): Arguments) -> ResolveResult {
    let separator = match args.first() {
        Some(Value::String(s)) => s.to_string(),
        _ => String::new(),
    };

    let parts: Vec<String> = this
        .iter()
        .map(|v| match v {
            Value::String(s) => s.to_string(),
            other => format!("{other:?}"),
        })
        .collect();

    Ok(Value::String(Arc::new(parts.join(&separator))))
}

/// `<string>.reverse() -> <string>`
///
/// Returns a new string with the characters in reverse order.
pub(crate) fn string_reverse(This(this): This<Arc<String>>) -> ResolveResult {
    let reversed: String = this.chars().rev().collect();
    Ok(Value::String(Arc::new(reversed)))
}

/// `strings.quote(<string>) -> <string>`
fn strings_quote(s: Arc<String>) -> ResolveResult {
    let mut escaped = String::with_capacity(s.len() + 2);
    escaped.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\x07' => escaped.push_str("\\a"),
            '\x08' => escaped.push_str("\\b"),
            '\x0C' => escaped.push_str("\\f"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\x0B' => escaped.push_str("\\v"),
            c => escaped.push(c),
        }
    }
    escaped.push('"');
    Ok(Value::String(Arc::new(escaped)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cel::Program;

    fn eval(expr: &str) -> Value {
        let mut ctx = Context::default();
        register(&mut ctx);
        // indexOf/lastIndexOf registered via dispatch
        crate::cel::dispatch::register(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap()
    }

    #[test]
    fn test_char_at() {
        assert_eq!(eval("'hello'.charAt(0)"), Value::String(Arc::new("h".into())));
        assert_eq!(eval("'hello'.charAt(4)"), Value::String(Arc::new("o".into())));
    }

    #[test]
    fn test_index_of() {
        assert_eq!(eval("'hello world'.indexOf('world')"), Value::Int(6));
        assert_eq!(eval("'hello'.indexOf('x')"), Value::Int(-1));
        assert_eq!(eval("'hello'.indexOf('')"), Value::Int(0));
    }

    #[test]
    fn test_last_index_of() {
        assert_eq!(eval("'abcabc'.lastIndexOf('abc')"), Value::Int(3));
        assert_eq!(eval("'hello'.lastIndexOf('x')"), Value::Int(-1));
    }

    #[test]
    fn test_lower_upper_ascii() {
        assert_eq!(
            eval("'Hello World'.lowerAscii()"),
            Value::String(Arc::new("hello world".into()))
        );
        assert_eq!(
            eval("'Hello World'.upperAscii()"),
            Value::String(Arc::new("HELLO WORLD".into()))
        );
    }

    #[test]
    fn test_trim() {
        assert_eq!(
            eval("'  hello  '.trim()"),
            Value::String(Arc::new("hello".into()))
        );
    }

    #[test]
    fn test_split() {
        assert_eq!(
            eval("'a,b,c'.split(',')"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("a".into())),
                Value::String(Arc::new("b".into())),
                Value::String(Arc::new("c".into())),
            ]))
        );
    }

    #[test]
    fn test_join() {
        assert_eq!(
            eval("['a', 'b', 'c'].join('-')"),
            Value::String(Arc::new("a-b-c".into()))
        );
    }

    #[test]
    fn test_replace() {
        assert_eq!(
            eval("'hello world'.replace('world', 'CEL')"),
            Value::String(Arc::new("hello CEL".into()))
        );
    }

    #[test]
    fn test_substring() {
        assert_eq!(
            eval("'hello'.substring(1)"),
            Value::String(Arc::new("ello".into()))
        );
    }

    #[test]
    fn test_strings_quote() {
        assert_eq!(
            eval("strings.quote('hello')"),
            Value::String(Arc::new("\"hello\"".into()))
        );
    }

    // --- Error & edge case tests ---

    fn eval_err(expr: &str) -> cel::ExecutionError {
        let mut ctx = Context::default();
        register(&mut ctx);
        crate::cel::dispatch::register(&mut ctx);
        Program::compile(expr).unwrap().execute(&ctx).unwrap_err()
    }

    #[test]
    fn test_char_at_out_of_bounds() {
        eval_err("'hello'.charAt(-1)");
        eval_err("'hello'.charAt(6)");
    }

    #[test]
    fn test_char_at_at_length() {
        // charAt(len) returns empty string (cel-go behavior)
        assert_eq!(eval("'hello'.charAt(5)"), Value::String(Arc::new("".into())));
        assert_eq!(eval("'tacocat'.charAt(7)"), Value::String(Arc::new("".into())));
    }

    #[test]
    fn test_char_at_unicode() {
        assert_eq!(eval("'héllo'.charAt(1)"), Value::String(Arc::new("é".into())));
    }

    #[test]
    fn test_index_of_with_offset() {
        // offset past first occurrence
        assert_eq!(eval("'abcabc'.indexOf('abc', 1)"), Value::Int(3));
        // negative offset clamps to 0
        assert_eq!(eval("'hello'.indexOf('h', -5)"), Value::Int(0));
        // offset past end
        assert_eq!(eval("'hello'.indexOf('h', 100)"), Value::Int(-1));
    }

    #[test]
    fn test_last_index_of_with_offset() {
        assert_eq!(eval("'abcabc'.lastIndexOf('abc', 3)"), Value::Int(0));
        // empty search returns the offset
        assert_eq!(eval("'hello'.lastIndexOf('', 3)"), Value::Int(3));
    }

    #[test]
    fn test_substring_two_args() {
        assert_eq!(
            eval("'hello'.substring(1, 3)"),
            Value::String(Arc::new("el".into()))
        );
    }

    #[test]
    fn test_substring_errors() {
        eval_err("'hello'.substring(-1)");
        eval_err("'hello'.substring(10)");
        eval_err("'hello'.substring(3, 2)"); // end < start
        eval_err("'hello'.substring(0, 10)"); // end > len
    }

    #[test]
    fn test_replace_with_count() {
        assert_eq!(
            eval("'aaa'.replace('a', 'b', 2)"),
            Value::String(Arc::new("bba".into()))
        );
        // count 0 replaces nothing
        assert_eq!(
            eval("'aaa'.replace('a', 'b', 0)"),
            Value::String(Arc::new("aaa".into()))
        );
    }

    #[test]
    fn test_split_with_limit() {
        assert_eq!(
            eval("'a,b,c'.split(',', 2)"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("a".into())),
                Value::String(Arc::new("b,c".into())),
            ]))
        );
    }

    #[test]
    fn test_join_no_separator() {
        assert_eq!(
            eval("['a', 'b', 'c'].join()"),
            Value::String(Arc::new("abc".into()))
        );
    }

    #[test]
    fn test_string_reverse() {
        assert_eq!(eval("'hello'.reverse()"), Value::String(Arc::new("olleh".into())));
        assert_eq!(eval("''.reverse()"), Value::String(Arc::new("".into())));
        assert_eq!(eval("'a'.reverse()"), Value::String(Arc::new("a".into())));
    }

    #[test]
    fn test_strings_quote_escapes() {
        assert_eq!(
            eval("strings.quote('a\\nb')"),
            Value::String(Arc::new("\"a\\nb\"".into()))
        );
        assert_eq!(
            eval("strings.quote('a\\tb')"),
            Value::String(Arc::new("\"a\\tb\"".into()))
        );
    }

    // --- cel-go parity tests ---

    #[test]
    fn test_char_at_unicode_multi() {
        assert_eq!(eval("'©αT'.charAt(0)"), Value::String(Arc::new("©".into())));
        assert_eq!(eval("'©αT'.charAt(1)"), Value::String(Arc::new("α".into())));
        assert_eq!(eval("'©αT'.charAt(2)"), Value::String(Arc::new("T".into())));
    }

    #[test]
    fn test_index_of_unicode() {
        assert_eq!(eval("'ta©o©αT'.indexOf('©')"), Value::Int(2));
        assert_eq!(eval("'ta©o©αT'.indexOf('©', 3)"), Value::Int(4));
        assert_eq!(eval("'ta©o©αT'.indexOf('©αT', 3)"), Value::Int(4));
    }

    #[test]
    fn test_index_of_full_match() {
        assert_eq!(eval("'hello wello'.indexOf('hello wello')"), Value::Int(0));
    }

    #[test]
    fn test_index_of_not_found_longer() {
        assert_eq!(eval("'hello wello'.indexOf('elbo room!!!')"), Value::Int(-1));
    }

    #[test]
    fn test_last_index_of_unicode() {
        assert_eq!(eval("'ta©o©αT'.lastIndexOf('©')"), Value::Int(4));
        assert_eq!(eval("'ta©o©αT'.lastIndexOf('©', 3)"), Value::Int(2));
    }

    #[test]
    fn test_last_index_of_empty_string() {
        assert_eq!(eval("''.lastIndexOf('@@')"), Value::Int(-1));
        assert_eq!(eval("'tacocat'.lastIndexOf('')"), Value::Int(7));
    }

    #[test]
    fn test_last_index_of_full_match() {
        assert_eq!(eval("'hello wello'.lastIndexOf('hello wello')"), Value::Int(0));
    }

    #[test]
    fn test_last_index_of_overlapping() {
        // lastIndexOf with offset limits search to positions [0, offset)
        assert_eq!(eval("'bananananana'.lastIndexOf('nana', 7)"), Value::Int(2));
        assert_eq!(eval("'bananananana'.lastIndexOf('nana')"), Value::Int(8));
    }

    #[test]
    fn test_replace_empty_pattern() {
        assert_eq!(
            eval("'hello hello'.replace('', '_')"),
            Value::String(Arc::new("_h_e_l_l_o_ _h_e_l_l_o_".into()))
        );
    }

    #[test]
    fn test_split_limit_zero() {
        // limit 0 returns empty list (cel-go behavior)
        assert_eq!(eval("'a,b,c'.split(',', 0)"), Value::List(Arc::new(vec![])));
    }

    #[test]
    fn test_split_negative_limit() {
        // negative limit returns all splits (cel-go behavior)
        assert_eq!(
            eval("'o©o©o©o'.split('©', -1)"),
            Value::List(Arc::new(vec![
                Value::String(Arc::new("o".into())),
                Value::String(Arc::new("o".into())),
                Value::String(Arc::new("o".into())),
                Value::String(Arc::new("o".into())),
            ]))
        );
    }

    #[test]
    fn test_substring_unicode() {
        assert_eq!(
            eval("'ta©o©αT'.substring(2, 6)"),
            Value::String(Arc::new("©o©α".into()))
        );
    }

    #[test]
    fn test_substring_at_end() {
        assert_eq!(
            eval("'ta©o©αT'.substring(7, 7)"),
            Value::String(Arc::new("".into()))
        );
    }

    #[test]
    fn test_lower_ascii_non_ascii_preserved() {
        // Non-ASCII characters should not be lowercased
        assert_eq!(
            eval("'TacoCÆt'.lowerAscii()"),
            Value::String(Arc::new("tacocÆt".into()))
        );
    }

    #[test]
    fn test_upper_ascii_non_ascii_preserved() {
        // Non-ASCII characters should not be uppercased
        assert_eq!(
            eval("'tacoCαt'.upperAscii()"),
            Value::String(Arc::new("TACOCαT".into()))
        );
    }

    #[test]
    fn test_strings_quote_special_escapes() {
        // \a (bell)
        let result = strings_quote(Arc::new("\x07".into())).unwrap();
        assert_eq!(result, Value::String(Arc::new("\"\\a\"".into())));
        // \b (backspace)
        let result = strings_quote(Arc::new("\x08".into())).unwrap();
        assert_eq!(result, Value::String(Arc::new("\"\\b\"".into())));
        // \f (form feed)
        let result = strings_quote(Arc::new("\x0C".into())).unwrap();
        assert_eq!(result, Value::String(Arc::new("\"\\f\"".into())));
        // \v (vertical tab)
        let result = strings_quote(Arc::new("\x0B".into())).unwrap();
        assert_eq!(result, Value::String(Arc::new("\"\\v\"".into())));
    }

    #[test]
    fn test_strings_quote_unicode_passthrough() {
        // Unicode and emoji should pass through unescaped
        let result = strings_quote(Arc::new("завтра".into())).unwrap();
        assert_eq!(result, Value::String(Arc::new("\"завтра\"".into())));
    }

    #[test]
    fn test_strings_quote_embedded_quote() {
        let result = strings_quote(Arc::new("mid string \" quote".into())).unwrap();
        assert_eq!(
            result,
            Value::String(Arc::new("\"mid string \\\" quote\"".into()))
        );
    }
}
