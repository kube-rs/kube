use darling::{
    FromDeriveInput, FromField, FromMeta,
    util::{IdentString, parse_expr},
};
use proc_macro2::TokenStream;
use syn::{Attribute, DeriveInput, Expr, Ident, Meta, Path, Token, parse_quote, punctuated::Punctuated};

#[derive(FromField)]
#[darling(attributes(x_kube))]
struct XKube {
    #[darling(multiple, rename = "validation", with = parse_expr::preserve_str_literal)]
    validations: Vec<Expr>,
    merge_strategy: Option<Expr>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(x_kube), supports(struct_named))]
struct KubeSchema {
    #[darling(default)]
    crates: Crates,
    ident: Ident,
    #[darling(multiple, rename = "validation", with = parse_expr::preserve_str_literal)]
    validations: Vec<Expr>,
}

#[derive(Debug, FromMeta)]
struct Crates {
    #[darling(default = "Self::default_kube_core")]
    kube_core: Path,
    #[darling(default = "Self::default_schemars")]
    schemars: Path,
    #[darling(default = "Self::default_serde")]
    serde: Path,
    #[darling(default = "Self::default_std")]
    std: Path,
}

// Default is required when the subattribute isn't mentioned at all
// Delegate to darling rather than deriving, so that we can piggyback off the `#[darling(default)]` clauses
impl Default for Crates {
    fn default() -> Self {
        Self::from_list(&[]).unwrap()
    }
}

impl Crates {
    fn default_kube_core() -> Path {
        parse_quote! { ::kube::core } // by default must work well with people using facade crate
    }

    fn default_schemars() -> Path {
        parse_quote! { ::schemars }
    }

    fn default_serde() -> Path {
        parse_quote! { ::serde }
    }

    fn default_std() -> Path {
        parse_quote! { ::std }
    }
}

pub(crate) fn derive_validated_schema(input: TokenStream) -> TokenStream {
    let mut ast: DeriveInput = match syn::parse2(input) {
        Err(err) => return err.to_compile_error(),
        Ok(di) => di,
    };

    let KubeSchema {
        crates:
            Crates {
                kube_core,
                schemars,
                serde,
                std,
            },
        ident,
        validations,
    } = match KubeSchema::from_derive_input(&ast) {
        Err(err) => return err.write_errors(),
        Ok(attrs) => attrs,
    };

    // Collect global structure validation rules
    let struct_name = IdentString::new(ident.clone()).map(|ident| format!("{ident}Validated"));
    let struct_rules: Vec<TokenStream> = validations
        .iter()
        .map(|rule| quote! {#kube_core::validate(s, #rule).unwrap();})
        .collect();

    // Modify generated struct name to avoid Struct::method conflicts in attributes
    ast.ident = struct_name.as_ident().clone();

    // Remove all unknown attributes from the original structure copy
    // Has to happen on the original definition at all times, as we don't have #[derive] stanzes.
    let attribute_whitelist = ["serde", "schemars", "doc", "validate"];
    ast.attrs = remove_attributes(&ast.attrs, &attribute_whitelist);

    // A bare container-level `#[serde(default)]` makes the schemars derive call
    // `<Self as Default>::default()` on the generated mirror structs, which have no `Default`
    // impl of their own. Emit delegating impls alongside the mirrors in that case.
    // (The `default = "path"` form needs none of this: schemars calls the function directly.)
    let original_default = has_container_serde_default(&ast.attrs)
        .then(|| quote! { <#ident as #std::default::Default>::default() });

    let struct_data = match ast.data {
        syn::Data::Struct(ref mut struct_data) => struct_data,
        _ => return quote! {},
    };

    // Preserve all serde attributes, to allow #[serde(rename_all = "camelCase")] or similar
    let struct_attrs: Vec<TokenStream> = ast.attrs.iter().map(|attr| quote! {#attr}).collect();
    let mut property_modifications = vec![];
    let mut validated_default_impl = quote! {};
    if let syn::Fields::Named(fields) = &mut struct_data.fields {
        if let Some(original) = &original_default {
            let field_idents: Vec<Ident> = fields.named.iter().filter_map(|f| f.ident.clone()).collect();
            let generated = struct_name.as_ident();
            validated_default_impl = quote! {
                #[automatically_derived]
                impl #std::default::Default for #generated {
                    fn default() -> Self {
                        let original = #original;
                        Self { #(#field_idents: original.#field_idents),* }
                    }
                }
            };
        }
        for field in &mut fields.named {
            let XKube {
                validations,
                merge_strategy,
                ..
            } = match XKube::from_field(field) {
                Ok(rule) => rule,
                Err(err) => return err.write_errors(),
            };

            // Remove all unknown attributes from each field
            // Has to happen on the original definition at all times, as we don't have #[derive] stanzes.
            field.attrs = remove_attributes(&field.attrs, &attribute_whitelist);

            if validations.is_empty() && merge_strategy.is_none() {
                continue;
            }

            let rules: Vec<TokenStream> = validations
                .iter()
                .map(|rule| quote! {#kube_core::validate_property(merge, 0, #rule).unwrap();})
                .collect();
            let merge_strategy = merge_strategy
                .map(|strategy| quote! {#kube_core::merge_strategy_property(merge, 0, #strategy).unwrap();});

            let field_default_impl = original_default.as_ref().map(|original| {
                let field_ident = &field.ident;
                quote! {
                    #[automatically_derived]
                    impl #std::default::Default for Validated {
                        fn default() -> Self {
                            let original = #original;
                            Self { #field_ident: original.#field_ident }
                        }
                    }
                }
            });

            // We need to prepend derive macros, as they were consumed by this macro processing, being a derive by itself.
            property_modifications.push(quote! {
                {
                    #[derive(#serde::Serialize, #schemars::JsonSchema)]
                    #(#struct_attrs)*
                    #[automatically_derived]
                    #[allow(missing_docs)]
                    struct Validated {
                        #field
                    }

                    #field_default_impl

                    let merge = &mut Validated::json_schema(generate);
                    #(#rules)*
                    #merge_strategy
                    #kube_core::merge_properties(s, merge);
                }
            });
        }
    }

    // The generated struct is renamed with a `Validated` suffix to avoid
    // colliding with the user's type, but that internal name must not become
    // the schema's public identity. These schemas are always inlined, so
    // `schema_name` is never used as a `$ref`; its only effect is the title
    // schemars emits at the schema root. Report the user's type name.
    let schema_name = ident.to_string();
    let generated_struct_name = struct_name.as_ident();

    quote! {
        impl #schemars::JsonSchema for #ident {
            fn inline_schema() -> bool {
                true
            }

            fn schema_name() -> #std::borrow::Cow<'static, str> {
                #schema_name.into()
            }

            fn json_schema(generate: &mut #schemars::generate::SchemaGenerator) -> schemars::Schema {
                #[derive(#serde::Serialize, #schemars::JsonSchema)]
                #[automatically_derived]
                #[allow(missing_docs)]
                #ast

                #validated_default_impl

                use #kube_core::{Rule, Message, Reason, ListMerge, MapMerge, StructMerge};
                let s = &mut #generated_struct_name::json_schema(generate);
                #(#struct_rules)*
                #(#property_modifications)*
                s.clone()
            }
        }
    }
}

// Detect a bare container-level `#[serde(default)]`.
// A bare `default` parses as `Meta::Path`; `default = "path"` is a `Meta::NameValue`
// and intentionally does not match (schemars resolves that function directly).
fn has_container_serde_default(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
        .filter_map(|attr| {
            attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
                .ok()
        })
        .flatten()
        .any(|meta| matches!(meta, Meta::Path(ref path) if path.is_ident("default")))
}

// Remove all unknown attributes from the list
fn remove_attributes(attrs: &[Attribute], witelist: &[&str]) -> Vec<Attribute> {
    attrs
        .iter()
        .filter(|attr| witelist.iter().any(|i| attr.path().is_ident(i)))
        .cloned()
        .collect()
}

#[test]
fn test_derive_validated() {
    let input = quote! {
        #[derive(CustomResource, KubeSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
        #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
        #[x_kube(validation = "self != ''")]
        struct FooSpec {
            #[x_kube(validation = "self != ''")]
            foo: String
        }
    };
    let input = syn::parse2(input).unwrap();
    let v = KubeSchema::from_derive_input(&input).unwrap();
    assert_eq!(v.validations.len(), 1);
}

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use syn::parse::{Parse as _, Parser as _};

    use super::*;
    #[test]
    fn test_derive_validated_full() {
        let input = quote! {
            #[derive(KubeSchema)]
            #[x_kube(validation = "true", validation = "false")]
            struct FooSpec {
                #[x_kube(validation = "true", validation = Rule::new("false"))]
                #[x_kube(merge_strategy = ListMerge::Atomic)]
                foo: Vec<String>
            }
        };

        let expected = quote! {
            impl ::schemars::JsonSchema for FooSpec {
                fn inline_schema() -> bool {
                    true
                }
                fn schema_name() -> ::std::borrow::Cow<'static, str> {
                    "FooSpec".into()
                }
                fn json_schema(
                    generate: &mut ::schemars::generate::SchemaGenerator,
                ) -> schemars::Schema {
                    #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                    #[automatically_derived]
                    #[allow(missing_docs)]
                    struct FooSpecValidated {
                        foo: Vec<String>,
                    }
                    use ::kube::core::{Rule, Message, Reason, ListMerge, MapMerge, StructMerge};
                    let s = &mut FooSpecValidated::json_schema(generate);
                    ::kube::core::validate(s, "true").unwrap();
                    ::kube::core::validate(s, "false").unwrap();
                    {
                        #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                        #[automatically_derived]
                        #[allow(missing_docs)]
                        struct Validated {
                            foo: Vec<String>,
                        }
                        let merge = &mut Validated::json_schema(generate);
                        ::kube::core::validate_property(merge, 0, "true").unwrap();
                        ::kube::core::validate_property(merge, 0, Rule::new("false")).unwrap();
                        ::kube::core::merge_strategy_property(merge, 0, ListMerge::Atomic).unwrap();
                        ::kube::core::merge_properties(s, merge);
                    }
                    s.clone()
                }
            }
        };

        let output = derive_validated_schema(input);
        let output = unparse(&syn::File::parse.parse2(output).unwrap());
        let expected = unparse(&syn::File::parse.parse2(expected).unwrap());
        assert_eq!(output, expected);
    }
}
