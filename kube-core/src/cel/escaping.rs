//! Field name escaping for Kubernetes CEL.
//!
//! Kubernetes CEL requires escaping JSON field names that collide with CEL
//! reserved words or contain special characters (`_`, `.`, `-`, `/`).
//! This module implements the same escaping rules as the Go apiserver
//! (`apiserver/schema/cel/model`).

/// CEL reserved words that must be escaped as `__keyword__`.
const CEL_RESERVED_WORDS: &[&str] = &[
    "true",
    "false",
    "null",
    "in",
    "as",
    "break",
    "const",
    "continue",
    "else",
    "for",
    "function",
    "if",
    "import",
    "let",
    "loop",
    "package",
    "namespace",
    "return",
    "var",
    "void",
    "while",
];

/// Escape a JSON field name for use as a CEL map key.
///
/// Rules (mutually exclusive, checked in order):
/// 1. If the name exactly matches a CEL reserved word → `__keyword__`
/// 2. If the name contains `_`, `.`, `-`, or `/` → character-level substitution
/// 3. Otherwise → return unchanged
///
/// Character substitutions (rule 2):
/// - `_` → `__`
/// - `.` → `__dot__`
/// - `-` → `__dash__`
/// - `/` → `__slash__`
#[must_use]
pub fn escape_field_name(name: &str) -> String {
    // Rule 1: exact match against reserved words
    if CEL_RESERVED_WORDS.contains(&name) {
        return format!("__{name}__");
    }

    // Rule 2: contains special characters
    if name.contains('_') || name.contains('.') || name.contains('-') || name.contains('/') {
        let mut escaped = String::with_capacity(name.len() * 2);
        for ch in name.chars() {
            match ch {
                '_' => escaped.push_str("__"),
                '.' => escaped.push_str("__dot__"),
                '-' => escaped.push_str("__dash__"),
                '/' => escaped.push_str("__slash__"),
                _ => escaped.push(ch),
            }
        }
        return escaped;
    }

    // Rule 3: no escaping needed
    name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_word_namespace() {
        assert_eq!(escape_field_name("namespace"), "__namespace__");
    }

    #[test]
    fn reserved_word_default_is_not_reserved() {
        // "default" is NOT in the CEL reserved word list
        assert_eq!(escape_field_name("default"), "default");
    }

    #[test]
    fn all_reserved_words() {
        for &word in CEL_RESERVED_WORDS {
            assert_eq!(escape_field_name(word), format!("__{word}__"));
        }
    }

    #[test]
    fn dash_escaping() {
        assert_eq!(escape_field_name("foo-bar"), "foo__dash__bar");
    }

    #[test]
    fn dot_escaping() {
        assert_eq!(escape_field_name("a.b"), "a__dot__b");
    }

    #[test]
    fn slash_escaping() {
        assert_eq!(escape_field_name("x/y"), "x__slash__y");
    }

    #[test]
    fn underscore_doubling() {
        assert_eq!(escape_field_name("my_field"), "my__field");
    }

    #[test]
    fn mixed_special_characters() {
        assert_eq!(escape_field_name("a-b_c.d"), "a__dash__b__c__dot__d");
    }

    #[test]
    fn no_escaping_needed() {
        assert_eq!(escape_field_name("replicas"), "replicas");
        assert_eq!(escape_field_name("spec"), "spec");
        assert_eq!(escape_field_name("fooBar"), "fooBar");
    }

    #[test]
    fn empty_string() {
        assert_eq!(escape_field_name(""), "");
    }

    #[test]
    fn reserved_word_takes_priority_over_special_chars() {
        // "in" is a reserved word — should get keyword escaping, not char-level
        assert_eq!(escape_field_name("in"), "__in__");
    }

    #[test]
    fn multiple_dashes() {
        assert_eq!(escape_field_name("a-b-c"), "a__dash__b__dash__c");
    }

    #[test]
    fn leading_underscore() {
        assert_eq!(escape_field_name("_private"), "__private");
    }

    #[test]
    fn slash_in_annotation_key() {
        assert_eq!(
            escape_field_name("app.kubernetes.io/name"),
            "app__dot__kubernetes__dot__io__slash__name"
        );
    }
}
