use crate::{CustomDerive, ResultExt};
use inflector::{cases::pascalcase::is_pascal_case, string::pluralize::to_plural};
use proc_macro2::{Ident, Span};
use syn::{Data, DeriveInput, Result, Visibility};

#[derive(Debug)]
pub struct CustomResource {
    tokens: proc_macro2::TokenStream,
    ident: proc_macro2::Ident,
    visibility: Visibility,
    kind: String,
    group: String,
    version: String,
    namespaced: bool,
    status: Option<String>,
    shortnames: Vec<String>,
    apiextensions: String,
    printcolums: Vec<String>,
    scale: Option<String>,
}

impl CustomDerive for CustomResource {
    fn parse(input: DeriveInput, tokens: proc_macro2::TokenStream) -> Result<Self> {
        let ident = input.ident;
        let visibility = input.vis;

        // Limit derive to structs
        let _s = match input.data {
            Data::Struct(ref s) => s,
            _ => return Err(r#"Enums or Unions can not #[derive(CustomResource)"#).spanning(ident),
        };

        // Outputs
        let mut group = None;
        let mut version = None;
        let mut namespaced = false;
        let mut status = None;
        let mut apiextensions = "v1".to_string();
        let mut scale = None;
        let mut printcolums = vec![];
        let mut shortnames = vec![];
        let mut kind = None;

        // Arg parsing
        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }
            if !attr.path.is_ident("kube") {
                continue;
            }
            let metas = match attr.parse_meta()? {
                syn::Meta::List(meta) => meta.nested,
                meta => return Err(r#"#[kube] expects a list of metas, like `#[kube(...)]`"#).spanning(meta),
            };

            for meta in metas {
                let meta: &dyn quote::ToTokens = match &meta {
                    // key-value arguments
                    syn::NestedMeta::Meta(syn::Meta::NameValue(meta)) => {
                        if meta.path.is_ident("group") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                group = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(group = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("version") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                version = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(version = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("scale") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                scale = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(scale = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("shortname") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                shortnames.push(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(shortname = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("kind") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                kind = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(scale = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("status") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                status = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(status = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("apiextensions") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                apiextensions = lit.value();
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(apiextensions = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else if meta.path.is_ident("printcolumn") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                printcolums.push(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(printcolumn = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else {
                            //println!("Unknown arg {:?}", meta.path.get_ident());
                            meta
                        }
                    }
                    // indicator arguments
                    syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                        if path.is_ident("namespaced") {
                            namespaced = true;
                            continue;
                        } else {
                            &meta
                        }
                    }

                    // unknown arg
                    meta => meta,
                };
                // throw on unknown arg
                return Err(r#"#[derive(CustomResource)] found unexpected meta"#).spanning(meta);
            }
        }

        // Find our Kind
        let struct_name = ident.to_string();
        let kind = if let Some(k) = kind {
            if k == struct_name {
                return Err(r#"#[derive(CustomResource)] `kind = "..."` must not equal the struct name (this is generated)"#)
                    .spanning(ident);
            }
            k
        } else {
            // Fallback, infer from struct name

            if !struct_name.ends_with("Spec") {
                return Err(r#"#[derive(CustomResource)] requires either a `kind = "..."` or the struct to end with `Spec`"#)
                    .spanning(ident);
            }
            struct_name[..(struct_name.len() - 4)].to_owned()
        };
        if !is_pascal_case(&kind) || to_plural(&kind) == kind {
            return Err(
                r#"#[derive(CustomResource)] requires a non-plural PascalCase `kind = "..."` or non-plural PascalCase struct name"#,
            )
            .spanning(ident);
        }

        let mkerror = |arg| {
            format!(
                r#"#[derive(CustomResource)] did not find a #[kube({} = "...")] attribute on the struct"#,
                arg
            )
        };
        let group = group.ok_or(mkerror("group")).spanning(&tokens)?;
        let version = version.ok_or(mkerror("version")).spanning(&tokens)?;

        Ok(CustomResource {
            tokens,
            ident,
            visibility,
            kind,
            group,
            version,
            namespaced,
            printcolums,
            status,
            shortnames,
            apiextensions,
            scale,
        })
    }

    // Using parsed info, create code
    fn emit(self) -> Result<proc_macro2::TokenStream> {
        let CustomResource {
            tokens,
            ident,
            visibility,
            group,
            kind,
            version,
            namespaced,
            status,
            shortnames,
            printcolums,
            apiextensions,
            scale,
        } = self;


        // 1. Create root object Foo and truncate name from FooSpec

        // Default visibility is `pub(crate)`
        // Default generics is no generics (makes little sense to re-use CRD kind?)
        // We enforce metadata + spec's existence (always there)
        // => No default impl
        let rootident = Ident::new(&kind, Span::call_site());

        // if status set, also add that
        let (statusq, statusdef) = if let Some(status_name) = &status {
            let ident = format_ident!("{}", status_name);
            let fst = quote! { #visibility status: Option<#ident>, };
            let snd = quote! { status: None, };
            (fst, snd)
        } else {
            let fst = quote! {};
            let snd = quote! {};
            (fst, snd)
        };
        let has_status = status.is_some();

        let root_obj = quote! {
            #[derive(Serialize, Deserialize, Clone, Debug)]
            #[serde(rename_all = "camelCase")]
            #visibility struct #rootident {
                #visibility api_version: String,
                #visibility kind: String,
                #visibility metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta,
                #visibility spec: #ident,
                #statusq
            }
            impl #rootident {
                pub fn new(name: &str, spec: #ident) -> Self {
                    Self {
                        api_version: <#rootident as k8s_openapi::Resource>::API_VERSION.to_string(),
                        kind: <#rootident as k8s_openapi::Resource>::KIND.to_string(),
                        metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                            name: Some(name.to_string()),
                            ..Default::default()
                        },
                        spec: spec,
                        #statusdef
                    }
                }
            }
        };

        // 2. Implement Resource trait for k8s_openapi
        let api_ver = format!("{}/{}", group, version);
        let impl_resource = quote! {
            impl k8s_openapi::Resource for #rootident {
                const API_VERSION: &'static str = #api_ver;
                const GROUP: &'static str = #group;
                const KIND: &'static str = #kind;
                const VERSION: &'static str = #version;
            }
        };

        // 3. Implement Metadata trait for k8s_openapi
        let impl_metadata = quote! {
            impl k8s_openapi::Metadata for #rootident {
                type Ty = k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
                fn metadata(&self) -> Option<&Self::Ty> {
                    Some(&self.metadata)
                }
            }
        };

        // 4. Implement CustomResource
        let name = kind.to_ascii_lowercase();
        let plural = to_plural(&name);
        let scope = if namespaced { "Namespaced" } else { "Cluster" };

        // Compute a bunch of crd props
        let mut printers = format!("[ {} ]", printcolums.join(",")); // hacksss
        if apiextensions == "v1beta1" {
            // only major api inconsistency..
            printers = printers.replace("jsonPath", "JSONPath");
        }
        let scale_code = if let Some(s) = scale { s } else { "".to_string() };

        // Ensure it generates for the correct CRD version
        let v1ident = format_ident!("{}", apiextensions);
        let apiext = quote! {
            k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::#v1ident
        };

        let short_json = format!(
            "[{}]",
            shortnames
                .into_iter()
                .map(|sn| format!("\"{}\"", sn))
                .collect::<Vec<_>>()
                .join(",")
        );
        let crd_meta_name = format!("{}.{}", plural, group);
        let crd_meta = quote! { { "name": #crd_meta_name } };
        // TODO: should ::crd be from a trait?
        let impl_crd = quote! {
            impl #rootident {
                pub fn crd() -> #apiext::CustomResourceDefinition {
                    let columns : Vec<#apiext::CustomResourceColumnDefinition> = serde_json::from_str(#printers).expect("valid printer column json");
                    let scale: Option<#apiext::CustomResourceSubresourceScale> = if #scale_code.is_empty() {
                        None
                    } else {
                        serde_json::from_str(#scale_code).expect("valid scale subresource json")
                    };
                    let shorts : Vec<String> = serde_json::from_str(#short_json).expect("valid shortnames");
                    let subres = if #has_status {
                        if let Some(s) = &scale {
                            serde_json::json!({
                                "status": {},
                                "scale": scale
                            })
                        } else {
                            serde_json::json!({"status": {} })
                        }
                    } else {
                        serde_json::json!({})
                    };

                    serde_json::from_value(serde_json::json!({
                        "metadata": #crd_meta,
                        "spec": {
                            "group": #group,
                            "scope": #scope,
                            "names": {
                                "plural": #plural,
                                "singular": #name,
                                "kind": #kind,
                                "shortNames": shorts
                            },
                            "versions": [{
                              "name": #version,
                              "served": true,
                              "storage": true,
                              "additionalPrinterColumns": columns,
                            }],
                            "subresources": subres,
                        }
                    })).expect("valid custom resource from #[kube(attrs..)]")
                }
            }
        };

        // Concat output
        let output = quote! {
            #root_obj
            #impl_resource
            #impl_metadata
            #impl_crd
        };
        // Try to convert to a TokenStream
        let res = syn::parse(output.into())
            .map_err(|err| format!("#[derive(CustomResource)] failed: {:?}", err))
            .spanning(&tokens)?;
        Ok(res)
    }
}
