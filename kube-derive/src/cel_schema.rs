use darling::{
    util::{parse_expr, IdentString},
    FromDeriveInput, FromField, FromMeta,
};
use proc_macro2::TokenStream;
use syn::{parse_quote, Attribute, DeriveInput, Expr, Ident, Path};

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

    let KubeSchema {
        crates: Crates {
            kube_core,
            schemars,
            serde,
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

    let struct_data = match ast.data {
        syn::Data::Struct(ref mut struct_data) => struct_data,
        _ => return quote! {},
    };

    // Preserve all serde attributes, to allow #[serde(rename_all = "camelCase")] or similar
    let struct_attrs: Vec<TokenStream> = ast.attrs.iter().map(|attr| quote! {#attr}).collect();
    let mut property_modifications = vec![];
    if let syn::Fields::Named(fields) = &mut struct_data.fields {
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
                    #(#rules)*
                    #merge_strategy
                    #kube_core::merge_properties(s, merge);
                }
            });
        }
    }

    let schema_name = struct_name.as_str();
    let generated_struct_name = struct_name.as_ident();

    quote! {
        impl #schemars::JsonSchema for #ident {
            fn is_referenceable() -> bool {
                false
            }

            fn schema_name() -> String {
                #schema_name.to_string()
            }

            fn json_schema(gen: &mut #schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                #[derive(#serde::Serialize, #schemars::JsonSchema)]
                #[automatically_derived]
                #[allow(missing_docs)]
                #ast

                use #kube_core::{Rule, Message, Reason, ListMerge, MapMerge, StructMerge};
                let s = &mut #generated_struct_name::json_schema(gen);
                #(#struct_rules)*
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
                fn is_referenceable() -> bool {
                    false
                }
                fn schema_name() -> String {
                    "FooSpecValidated".to_string()
                }
                fn json_schema(
                    gen: &mut ::schemars::gen::SchemaGenerator,
                ) -> schemars::schema::Schema {
                    #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                    #[automatically_derived]
                    #[allow(missing_docs)]
                    struct FooSpecValidated {
                        foo: Vec<String>,
                    }
                    use ::kube::core::{Rule, Message, Reason, ListMerge, MapMerge, StructMerge};
                    let s = &mut FooSpecValidated::json_schema(gen);
                    ::kube::core::validate(s, "true").unwrap();
                    ::kube::core::validate(s, "false").unwrap();
                    {
                        #[derive(::serde::Serialize, ::schemars::JsonSchema)]
                        #[automatically_derived]
                        #[allow(missing_docs)]
                        struct Validated {
                            foo: Vec<String>,
                        }
                        let merge = &mut Validated::json_schema(gen);
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
