use darling::{FromDeriveInput, FromField, FromMeta};
use proc_macro2::TokenStream;
use syn::{parse_quote, Attribute, DeriveInput, Expr, Ident, Path};

#[derive(FromField)]
#[darling(attributes(cel_validate))]
struct Rule {
    #[darling(multiple, rename = "rule")]
    rules: Vec<Expr>,
}

#[derive(FromDeriveInput)]
#[darling(attributes(cel_validate), supports(struct_named))]
struct CELSchema {
    #[darling(default)]
    crates: Crates,
    ident: Ident,
    #[darling(multiple, rename = "rule")]
    rules: Vec<Expr>,
}

#[derive(Debug, FromMeta)]
struct Crates {
    #[darling(default = "Self::default_kube_core")]
    kube_core: Path,
    #[darling(default = "Self::default_schemars")]
    schemars: Path,
    #[darling(default = "Self::default_serde")]
    serde: Path,
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
}

pub(crate) fn derive_validated_schema(input: TokenStream) -> TokenStream {
    let mut ast: DeriveInput = match syn::parse2(input) {
        Err(err) => return err.to_compile_error(),
        Ok(di) => di,
    };

    let CELSchema {
        crates: Crates {
            kube_core,
            schemars,
            serde,
        },
        ident,
        rules,
    } = match CELSchema::from_derive_input(&ast) {
        Err(err) => return err.write_errors(),
        Ok(attrs) => attrs,
    };

    // Collect global structure validation rules
    let struct_name = ident.to_string();
    let struct_rules: Vec<TokenStream> = rules.iter().map(|r| quote! {#r,}).collect();

    // Remove all unknown attributes from the original structure copy
    // Has to happen on the original definition at all times, as we don't have #[derive] stanzes.
    let attribute_whitelist = ["serde", "schemars", "doc"];
    ast.attrs = remove_attributes(&ast.attrs, &attribute_whitelist);

    let struct_data = match ast.data {
        syn::Data::Struct(ref mut struct_data) => struct_data,
        _ => return quote! {},
    };

    // Preserve all serde attributes, to allow #[serde(rename_all = "camelCase")] or similar
    let struct_attrs: Vec<TokenStream> = ast.attrs.iter().map(|attr| quote! {#attr}).collect();
    let mut property_modifications = vec![];
    if let syn::Fields::Named(fields) = &mut struct_data.fields {
        for field in &mut fields.named {
            let Rule { rules, .. } = match Rule::from_field(field) {
                Ok(rule) => rule,
                Err(err) => return err.write_errors(),
            };

            // Remove all unknown attributes from each field
            // Has to happen on the original definition at all times, as we don't have #[derive] stanzes.
            field.attrs = remove_attributes(&field.attrs, &attribute_whitelist);

            if rules.is_empty() {
                continue;
            }

            let rules: Vec<TokenStream> = rules.iter().map(|r| quote! {#r,}).collect();

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

                    let merge = &mut Validated::json_schema(gen);
                    #kube_core::validate_property(merge, 0, &[#(#rules)*]).unwrap();
                    #kube_core::merge_properties(s, merge);
                }
            });
        }
    }

    quote! {
        impl #schemars::JsonSchema for #ident {
            fn is_referenceable() -> bool {
                false
            }

            fn schema_name() -> String {
                #struct_name.to_string() + "_kube_validation".into()
            }

            fn json_schema(gen: &mut #schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                #[derive(#serde::Serialize, #schemars::JsonSchema)]
                #[automatically_derived]
                #[allow(missing_docs)]
                #ast

                use #kube_core::{Rule, Message, Reason};
                let s = &mut #ident::json_schema(gen);
                #kube_core::validate(s, &[#(#struct_rules)*]).unwrap();
                #(#property_modifications)*
                s.clone()
            }
        }
    }
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
        #[derive(CustomResource, CELSchema, Serialize, Deserialize, Debug, PartialEq, Clone)]
        #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
        #[cel_validate(rule = "self != ''".into())]
        struct FooSpec {
            #[cel_validate(rule = "self != ''".into())]
            foo: String
        }
    };
    let input = syn::parse2(input).unwrap();
    let v = CELSchema::from_derive_input(&input).unwrap();
    assert_eq!(v.rules.len(), 1);
}

#[cfg(test)]
mod tests {
    use prettyplease::unparse;
    use syn::parse::{Parse as _, Parser as _};

    use super::*;
    #[test]
    fn test_derive_validated_full() {
        let input = quote! {
            #[derive(CELSchema)]
            #[cel_validate(rule = "true".into())]
            struct FooSpec {
                #[cel_validate(rule = "true".into())]
                foo: String
            }
        };

        let expected = quote! {
            impl ::schemars::JsonSchema for FooSpec {
                fn is_referenceable() -> bool {
                    false
                }
                fn schema_name() -> String {
                    "FooSpec".to_string() + "_kube_validation".into()
                }
                fn json_schema(
                    gen: &mut ::schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                    #[automatically_derived]
                    #[allow(missing_docs)]
                    struct FooSpec {
                        foo: String,
                    }
                    use ::kube::core::{Rule, Message, Reason};
                    let s = &mut FooSpec::json_schema(gen);
                    ::kube::core::validate(s, &["true".into()]).unwrap();
                    {
                        #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                        #[automatically_derived]
                        #[allow(missing_docs)]
                        struct Validated {
                            foo: String,
                        }
                        let merge = &mut Validated::json_schema(gen);
                        ::kube::core::validate_property(merge, 0, &["true".into()]).unwrap();
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
