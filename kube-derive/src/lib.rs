#![recursion_limit = "1024"]
#![warn(rust_2018_idioms)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::too_many_lines)]

extern crate proc_macro;

trait CustomDerive: Sized {
    fn parse(input: syn::DeriveInput, tokens: proc_macro2::TokenStream) -> Result<Self, syn::Error>;
    fn emit(self) -> Result<proc_macro2::TokenStream, syn::Error>;
}

fn run_custom_derive<T>(input: proc_macro::TokenStream) -> proc_macro::TokenStream
where
    T: CustomDerive,
{
    let input: proc_macro2::TokenStream = input.into();
    let tokens = input.clone();
    let token_stream = match syn::parse2(input)
        .and_then(|input| <T as CustomDerive>::parse(input, tokens))
        .and_then(<T as CustomDerive>::emit)
    {
        Ok(token_stream) => token_stream,
        Err(err) => err.to_compile_error(),
    };
    token_stream.into()
}

trait ResultExt<T> {
    fn spanning(self, spanned: impl quote::ToTokens) -> Result<T, syn::Error>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn spanning(self, spanned: impl quote::ToTokens) -> Result<T, syn::Error> {
        self.map_err(|err| syn::Error::new_spanned(spanned, err))
    }
}

/// A custom derive for kubernetes custom resource definitions.
///
/// This will implement the `k8s_openapi::Metadata` and `k8s_openapi::Resource` traits
/// so the type can be used with the `kube` crate.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(CustomResource, Clone, Debug, PartialEq, Deserialize, Serialize)]
/// #[kube(group = "clux.dev", version = "v1", plural = "foos", namespaced)]
/// struct FooSpec {
///     prop1: String,
///     prop2: Vec<bool>,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     prop3: Option<i32>,
/// }
/// ```
/// Try `cargo-expand` to see your own macro expansion.

#[proc_macro_derive(CustomResource, attributes(kube))]
pub fn derive_custom_resource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_custom_derive::<CustomResource>(input)
}

#[derive(Debug)]
struct CustomResource {
    ident: proc_macro2::Ident,
    vis: syn::Visibility,
    tokens: proc_macro2::TokenStream,

    group: String,
    version: String,
    plural: String,
    namespaced: bool,
}

// TODO: create root object with ObjectMeta? if so, truncate to prefix (FooSpec -> Foo)
/// #[derive(Clone, Debug, Default, PartialEq)]
/// struct FooBar {
///     metadata: Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta>,
///     spec: Option<FooSpec>,
/// }
// maybe we should force root type created? and enforce metadata..

// TODO: impl k8s_openapi::Resource for the root type
// TODO: impl k8s_openapi::Metadata<Ty=ObjectMeta> for the root type
// impl serialize in the normal way...

// TODO: infer plural uring Inflector lib (already depend on it)

// TODO: create CRD spec? `apiextensions::CustomResourceDefinition`
// ^ bother? just do what kubebuilder does for now and chuck json at it..
// We should define a trait that can return apiextensions::CustomResourceDefinition ?
// then impl that trait for the the root type?

/// let custom_resource_spec = apiextensions::CustomResourceDefinitionSpec {
///     group: <FooBar as k8s_openapi::Resource>::GROUP.to_owned(),
///     names: apiextensions::CustomResourceDefinitionNames {
///         kind: <FooBar as k8s_openapi::Resource>::KIND.to_owned(),
///         plural: plural.to_owned(),
///         short_names: Some(vec!["fb".to_owned()]),
///         singular: Some("foobar".to_owned()),
///         ..Default::default()
///     },
///     scope: "Namespaced".to_owned(),
///     version: <FooBar as k8s_openapi::Resource>::VERSION.to_owned().into(),
///     ..Default::default()
/// };

/// let custom_resource = apiextensions::CustomResourceDefinition {
///     metadata: Some(meta::ObjectMeta {
///         name: Some(format!("{}.{}", plural, <FooBar as k8s_openapi::Resource>::GROUP)),
///         ..Default::default()
///     }),
///     spec: custom_resource_spec.into(),
///     ..Default::default()
/// };
///

impl CustomDerive for CustomResource {
    fn parse(input: syn::DeriveInput, tokens: proc_macro2::TokenStream) -> Result<Self, syn::Error> {
        let ident = input.ident;
        let vis = input.vis;

        let mut group = None;
        let mut plural = None;
        let mut version = None;

        let mut namespaced = false;

        for attr in &input.attrs {
            if attr.style != syn::AttrStyle::Outer {
                continue;
            }

            if !attr.path.is_ident("kube") {
                continue;
            }

            let metas = match attr.parse_meta()? {
                syn::Meta::List(meta) => meta.nested,
                meta => {
                    return Err(
                        r#"#[kube] expects a list of metas, like `#[kube(...)]`"#,
                    )
                    .spanning(meta)
                }
            };

            for meta in metas {
                let meta: &dyn quote::ToTokens = match &meta {
                    syn::NestedMeta::Meta(syn::Meta::NameValue(meta)) => {
                        if meta.path.is_ident("group") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                group = Some(lit.value());
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(group = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else if meta.path.is_ident("plural") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                plural = Some(lit.value());
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(plural = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else if meta.path.is_ident("version") {
                            if let syn::Lit::Str(lit) = &meta.lit {
                                version = Some(lit.value());
                                continue;
                            } else {
                                return Err(
                                    r#"#[kube(version = "...")] expects a string literal value"#,
                                )
                                .spanning(meta);
                            }
                        } else {
                            meta
                        }
                    }

                    syn::NestedMeta::Meta(syn::Meta::Path(path)) => {
                        if path.is_ident("namespaced") {
                            namespaced = true;
                            continue;
                        } else {
                            &meta
                        }
                    }

                    meta => meta,
                };

                return
                    Err(r#"#[derive(CustomResource)] found unexpected meta. Expected `group = "..."`, `namespaced`, `plural = "..."` or `version = "..."`"#)
                    .spanning(meta);
            }
        }

        let group =
            group
            .ok_or(r#"#[derive(CustomResource)] did not find a #[kube(group = "...")] attribute on the struct"#)
            .spanning(&tokens)?;
        let version =
            version
            .ok_or(r#"#[derive(CustomResource)] did not find a #[kube(version = "...")] attribute on the struct"#)
            .spanning(&tokens)?;
        let plural =
            plural
            .ok_or(r#"#[derive(CustomResource)] did not find a #[kube(plural = "...")] attribute on the struct"#)
            .spanning(&tokens)?;

        Ok(CustomResource {
            ident,
            vis,
            tokens,

            group,
            version,
            namespaced,
            plural,
        })
    }

    fn emit(self) -> Result<proc_macro2::TokenStream, syn::Error> {
        let CustomResource {
            ident: cr_spec_name,
            vis,
            tokens,
            group,
            version,
            plural,
            namespaced,
        } = self;

        let vis: std::borrow::Cow<'_, str> = match vis {
            syn::Visibility::Inherited => "".into(),
            vis => format!("{} ", quote::ToTokens::into_token_stream(vis)).into(),
        };

        let (cr_spec_name, cr_name) = {
            let cr_spec_name_string = cr_spec_name.to_string();
            if !cr_spec_name_string.ends_with("Spec") {
                return Err("#[derive(CustomResource)] requires the name of the struct to end with `Spec`")
                    .spanning(cr_spec_name);
            }
            let cr_name_string = cr_spec_name_string[..(cr_spec_name_string.len() - 4)].to_owned();
            (cr_spec_name_string, cr_name_string)
        };

        let mut out = vec![];

        let out = String::from_utf8(out)
            .map_err(|err| format!("#[derive(CustomResource)] failed: {}", err))
            .spanning(&tokens)?;
        let result = out
            .parse()
            .map_err(|err| format!("#[derive(CustomResource)] failed: {:?}", err))
            .spanning(&tokens)?;
        Ok(result)
    }
}
