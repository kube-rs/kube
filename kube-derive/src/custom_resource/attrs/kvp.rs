use darling::FromMeta;

#[derive(Debug)]
pub struct KeyValuePair(pub String, pub String);

impl FromMeta for KeyValuePair {
    fn from_list(items: &[darling::ast::NestedMeta]) -> darling::Result<Self> {
        if let [
            darling::ast::NestedMeta::Lit(syn::Lit::Str(key)),
            darling::ast::NestedMeta::Lit(syn::Lit::Str(value)),
        ] = items
        {
            return Ok(KeyValuePair(key.value(), value.value()));
        }

        Err(darling::Error::unsupported_format(
            "expected `\"key\", \"value\"` format",
        ))
    }
}

impl From<(&'static str, &'static str)> for KeyValuePair {
    fn from((key, value): (&'static str, &'static str)) -> Self {
        Self(key.to_string(), value.to_string())
    }
}
