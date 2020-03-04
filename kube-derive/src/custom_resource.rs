use crate::{CustomDerive, ResultExt};
use proc_macro2::{Ident, Span};
use syn::{Data, DeriveInput, Result};

#[derive(Debug)]
pub struct CustomResource {
    tokens: proc_macro2::TokenStream,
    ident: proc_macro2::Ident,
    kind: String,
    group: String,
    version: String,
    namespaced: bool,
    status: bool,
    crd_version: String,
    printcolums: Vec<String>,
    scale: Option<String>,
}

impl CustomDerive for CustomResource {
    fn parse(input: DeriveInput, tokens: proc_macro2::TokenStream) -> Result<Self> {
        let ident = input.ident;

        // Limit derive to structs
        let _s = match input.data {
            Data::Struct(ref s) => s,
            _ => return Err(r#"Enums or Unions can not #[derive(CustomResource)"#).spanning(ident),
        };

        // Parse struct name. Must end in Spec and be PascalCase
        let kind = {
            let struct_name = ident.to_string();
            if !struct_name.ends_with("Spec") {
                return Err("#[derive(CustomResource)] requires the name of the struct to end with `Spec`")
                    .spanning(ident);
            }
            struct_name[..(struct_name.len() - 4)].to_owned()
        };

        // Outputs
        let mut group = None;
        let mut version = None;
        let mut namespaced = false;
        let mut status = false;
        let mut crd_version = "v1".to_string();
        let mut scale = None;
        let mut printcolums = vec![];
        // TODO:
        // #[kube(subresource:status = FooStatus)] ?

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
                        } else if meta.path.is_ident("subresource_scale") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                scale = Some(lit.value());
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(subresource_scale = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else if meta.path.is_ident("crd_version") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                crd_version = lit.value();
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(crd_version = "...")] expects a string literal value"#,
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
                            meta
                        }
                    }
                    // indicator arguments
                    syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                        if path.is_ident("namespaced") {
                            namespaced = true;
                            continue;
                        } else if path.is_ident("subresource_status") {
                            status = true;
                            continue;
                        } else {
                            &meta
                        }
                    }

                    // unknown arg
                    meta => meta,
                };
                // throw on unknown arg
                return
                    Err(r#"#[derive(CustomResource)] found unexpected meta. Expected `group = "..."`, `namespaced`, or `version = "..."`"#)
                    .spanning(meta);
            }
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
            kind,
            group,
            version,
            namespaced,
            printcolums,
            status,
            crd_version,
            scale,
        })
    }

    // Using parsed info, create code
    fn emit(self) -> Result<proc_macro2::TokenStream> {
        let CustomResource {
            tokens,
            ident,
            group,
            kind,
            version,
            namespaced,
            status,
            printcolums,
            crd_version,
            scale,
        } = self;


        // 1. Create root object Foo and truncate name from FooSpec

        // Default visibility is `pub(crate)`
        // Default generics is no generics (makes little sense to re-use CRD kind?)
        // We enforce metadata + spec's existence (always there)
        // => No default impl
        let rootident = Ident::new(&kind, Span::call_site());

        // if status set, also add that
        let statusq = if status {
            let ident = format_ident!("{}{}", kind, "Status");
            quote! { status: Option<#ident>, }
        } else {
            quote! {}
        };

        let root_obj = quote! {
            #[derive(Serialize, Deserialize, Debug, Clone)]
            pub struct #rootident {
                metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta,
                spec: #ident,
                #statusq
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
        use inflector::string::pluralize::to_plural;
        let name = kind.to_ascii_lowercase();
        let plural = to_plural(&name);
        let scope = if namespaced { "Namespaced" } else { "Cluster" };

        // TODO: verify serialize at compile time vs current runtime check..
        // HOWEVER.. That requires k8s_openapi dep in here..
        // and we need to define the version feature :/
        // ... this will clash with user selected feature :(
        // Sketch:
        //use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
        //let crd : CustomResourceDefinition = serde_json::from_value(
        //    serde_json::json!({})
        //    ).map_err(|e| format!(r#"#[derive(CustomResource)] was unable to deserialize CustomResourceDefinition: {}"#, e))
        //.spanning(&tokens)?;
        //let crd_json = serde_json::to_string(&crd).unwrap();

        // Compute a bunch of crd props
        let mut printers = format!("[ {} ]", printcolums.join(",")); // hacksss
        if crd_version == "v1beta1" {
            // only major api inconsistency..
            printers = printers.replace("jsonPath", "JSONPath");
        }
        let scale_code = if let Some(s) = scale { s } else { "".to_string() };

        // Ensure it generates for the correct CRD version
        let v1ident = format_ident!("{}", crd_version);
        let use_correct_crd = quote !{
            use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::#v1ident as apiext;
        };

        let crd_meta_name = format!("{}.{}", plural, group);
        let crd_meta = quote! { { "name": #crd_meta_name } };
        let impl_crd = quote! {
            #use_correct_crd
            impl #rootident {
                fn crd() -> apiext::CustomResourceDefinition {
                    trace!("Printers: {}, Scale: {}", #printers, #scale_code);
                    let columns : Vec<apiext::CustomResourceColumnDefinition> = serde_json::from_str(#printers).expect("valid printer column json");
                    let scale: Option<apiext::CustomResourceSubresourceScale> = if #scale_code.is_empty() {
                        None
                    } else {
                        serde_json::from_str(#scale_code).expect("valid scale subresource json")
                    };
                    let subres = if #status {
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
