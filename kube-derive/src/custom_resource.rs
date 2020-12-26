use crate::{CustomDerive, ResultExt};
use inflector::string::pluralize::to_plural;
use proc_macro2::{Ident, Span};
use syn::{Data, DeriveInput, Path, Result, Visibility};

#[derive(Debug)]
pub(crate) struct CustomResource {
    tokens: proc_macro2::TokenStream,
    ident: proc_macro2::Ident,
    visibility: Visibility,
    kubeattrs: KubeAttrs,
}

/// Values we can parse from #[kube(attrs)]
#[derive(Debug, Default)]
struct KubeAttrs {
    group: String,
    version: String,
    kind: String,
    kind_struct: String,
    /// lowercase plural of kind (inferred if omitted)
    plural: Option<String>,
    namespaced: bool,
    apiextensions: String,
    derives: Vec<String>,
    status: Option<String>,
    shortnames: Vec<String>,
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
        let mut ka = KubeAttrs::default();
        let (mut group, mut version, mut kind) = (None, None, None); // mandatory GVK
        let mut kind_struct = None;
        ka.apiextensions = "v1".to_string(); // implicit stable crd version expected

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
                        } else if meta.path.is_ident("kind") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                kind = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(kind = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("struct") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                kind_struct = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(struct = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("plural") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.plural = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(plural = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("shortname") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.shortnames.push(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(shortname = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("scale") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.scale = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(scale = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("status") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.status = Some(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(status = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("apiextensions") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.apiextensions = lit.value();
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(apiextensions = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else if meta.path.is_ident("printcolumn") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.printcolums.push(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(printcolumn = "...")] expects a string literal value"#)
                                    .spanning(meta);
                            }
                        } else if meta.path.is_ident("derive") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                ka.derives.push(lit.value());
                                continue;
                            } else {
                                return Err(r#"#[kube(derive = "...")] expects a string literal value"#)
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
                            ka.namespaced = true;
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

        // Unpack the mandatory GVK
        let mkerror = |arg| {
            format!(
                r#"#[derive(CustomResource)] did not find a #[kube({} = "...")] attribute on the struct"#,
                arg
            )
        };
        ka.group = group.ok_or_else(|| mkerror("group")).spanning(&tokens)?;
        ka.version = version.ok_or_else(|| mkerror("version")).spanning(&tokens)?;
        ka.kind = kind.ok_or_else(|| mkerror("kind")).spanning(&tokens)?;
        ka.kind_struct = kind_struct.unwrap_or_else(|| ka.kind.clone());

        let struct_name = ident.to_string();
        if ka.kind_struct == struct_name {
            return Err(r#"#[derive(CustomResource)] `kind = "..."` must not equal the struct name (this is generated)"#)
                    .spanning(ident);
        }
        Ok(CustomResource {
            kubeattrs: ka,
            tokens,
            ident,
            visibility,
        })
    }

    // Using parsed info, create code
    fn emit(self) -> Result<proc_macro2::TokenStream> {
        let CustomResource {
            tokens,
            ident,
            visibility,
            kubeattrs,
        } = self;

        let KubeAttrs {
            group,
            kind,
            kind_struct,
            version,
            namespaced,
            derives,
            status,
            plural,
            shortnames,
            printcolums,
            apiextensions,
            scale,
        } = kubeattrs;

        // 1. Create root object Foo and truncate name from FooSpec

        // Default visibility is `pub(crate)`
        // Default generics is no generics (makes little sense to re-use CRD kind?)
        // We enforce metadata + spec's existence (always there)
        // => No default impl
        let rootident = Ident::new(&kind_struct, Span::call_site());

        // if status set, also add that
        let (statusq, statusdef) = if let Some(status_name) = &status {
            let ident = format_ident!("{}", status_name);
            let fst = quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                #visibility status: Option<#ident>,
            };
            let snd = quote! { status: None, };
            (fst, snd)
        } else {
            let fst = quote! {};
            let snd = quote! {};
            (fst, snd)
        };
        let has_status = status.is_some();
        let mut has_default = false;

        let mut derive_paths: Vec<Path> = vec![];
        for d in ["::serde::Serialize", "::serde::Deserialize", "Clone", "Debug"].iter() {
            derive_paths.push(syn::parse_str(*d)?);
        }
        for d in &derives {
            if d == "Default" {
                has_default = true; // overridden manually to avoid confusion
            } else {
                derive_paths.push(syn::parse_str(d)?);
            }
        }

        // Schema generation is always enabled for v1 because it's mandatory.
        // TODO Enable schema generation for v1beta1 if the spec derives `JsonSchema`.
        let schema_gen_enabled = apiextensions == "v1" && cfg!(feature = "schema");
        // We exclude fields `apiVersion`, `kind`, and `metadata` from our schema because
        // these are validated by the API server implicitly. Also, we can't generate the
        // schema for `metadata` (`ObjectMeta`) because it doesn't implement `JsonSchema`.
        let schemars_skip = if schema_gen_enabled {
            quote! { #[schemars(skip)] }
        } else {
            quote! {}
        };
        if schema_gen_enabled {
            derive_paths.push(syn::parse_str("::schemars::JsonSchema")?);
        }

        let docstr = format!(" Auto-generated derived type for {} via `CustomResource`", ident);
        let root_obj = quote! {
            #[doc = #docstr]
            #[derive(#(#derive_paths),*)]
            #[serde(rename_all = "camelCase")]
            #visibility struct #rootident {
                #schemars_skip
                #visibility api_version: String,
                #schemars_skip
                #visibility kind: String,
                #schemars_skip
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
                fn metadata(&self) -> &Self::Ty {
                    &self.metadata
                }
                fn metadata_mut(&mut self) -> &mut Self::Ty {
                    &mut self.metadata
                }
            }
        };
        // 4. Implement Default if requested
        let impl_default = if has_default {
            quote! {
                impl Default for #rootident {
                    fn default() -> Self {
                        Self {
                            api_version: <#rootident as k8s_openapi::Resource>::API_VERSION.to_string(),
                            kind: <#rootident as k8s_openapi::Resource>::KIND.to_string(),
                            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default(),
                            spec: Default::default(),
                            #statusdef
                        }
                    }
                }
            }
        } else {
            quote! {}
        };

        // 5. Implement CustomResource
        let name = kind.to_ascii_lowercase();
        let plural = plural.unwrap_or_else(|| to_plural(&name));
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

        let short_json = serde_json::to_string(&shortnames).unwrap();
        let crd_meta_name = format!("{}.{}", plural, group);
        let crd_meta = quote! { { "name": #crd_meta_name } };

        let schemagen = if schema_gen_enabled {
            quote! {
                // Don't use definitions and don't include `$schema` because these are not allowed.
                let gen = schemars::gen::SchemaSettings::openapi3().with(|s| {
                    s.inline_subschemas = true;
                    s.meta_schema = None;
                }).into_generator();
                let schema = gen.into_root_schema_for::<Self>();
            }
        } else {
            // we could issue a compile time warning for this, but it would hit EVERY compile, which would be noisy
            // eprintln!("warning: kube-derive configured with manual schema generation");
            // users must manually set a valid schema in crd.spec.versions[*].schema - see examples: crd_derive_no_schema
            quote! {
                let schema: Option<k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::JSONSchemaProps> = None;
            }
        };

        let jsondata = if apiextensions == "v1" {
            quote! {
                #schemagen

                let jsondata = serde_json::json!({
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
                            "schema": {
                                "openAPIV3Schema": schema,
                            },
                            "additionalPrinterColumns": columns,
                            "subresources": subres,
                        }],
                    }
                });
            }
        } else {
            // TODO Include schema if enabled
            quote! {
                let jsondata = serde_json::json!({
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
                        // printer columns can't be on versions reliably in v1beta..
                        "additionalPrinterColumns": columns,
                        "versions": [{
                            "name": #version,
                            "served": true,
                            "storage": true,
                        }],
                        "subresources": subres,
                    }
                });
            }
        };

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

                    #jsondata
                    serde_json::from_value(jsondata)
                        .expect("valid custom resource from #[kube(attrs..)]")
                }
            }
        };

        // Concat output
        let output = quote! {
            #root_obj
            #impl_resource
            #impl_metadata
            #impl_default
            #impl_crd
        };
        // Try to convert to a TokenStream
        let res = syn::parse(output.into())
            .map_err(|err| format!("#[derive(CustomResource)] failed: {:?}", err))
            .spanning(&tokens)?;
        Ok(res)
    }
}
