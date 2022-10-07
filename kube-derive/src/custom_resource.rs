use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{parse_quote, Data, DeriveInput, Path, Visibility};

/// Values we can parse from #[kube(attrs)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(kube))]
struct KubeAttrs {
    group: String,
    version: String,
    kind: String,
    #[darling(rename = "struct")]
    kind_struct: Option<String>,
    /// lowercase plural of kind (inferred if omitted)
    plural: Option<String>,
    /// singular defaults to lowercased kind
    singular: Option<String>,
    #[darling(default)]
    namespaced: bool,
    #[darling(multiple, rename = "derive")]
    derives: Vec<String>,
    schema: Option<SchemaMode>,
    status: Option<String>,
    #[darling(multiple, rename = "category")]
    categories: Vec<String>,
    #[darling(multiple, rename = "shortname")]
    shortnames: Vec<String>,
    #[darling(multiple, rename = "printcolumn")]
    printcolums: Vec<String>,
    scale: Option<String>,
    #[darling(default)]
    crates: Crates,
}

#[derive(Debug, FromMeta)]
struct Crates {
    #[darling(default = "Self::default_kube_core")]
    kube_core: Path,
    #[darling(default = "Self::default_k8s_openapi")]
    k8s_openapi: Path,
    #[darling(default = "Self::default_schemars")]
    schemars: Path,
    #[darling(default = "Self::default_serde")]
    serde: Path,
    #[darling(default = "Self::default_serde_json")]
    serde_json: Path,
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

    fn default_k8s_openapi() -> Path {
        parse_quote! { ::k8s_openapi }
    }

    fn default_schemars() -> Path {
        parse_quote! { ::schemars }
    }

    fn default_serde() -> Path {
        parse_quote! { ::serde }
    }

    fn default_serde_json() -> Path {
        parse_quote! { ::serde_json }
    }

    fn default_std() -> Path {
        parse_quote! { ::std }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum SchemaMode {
    Disabled,
    Manual,
    Derived,
}

impl SchemaMode {
    fn derive(self) -> bool {
        match self {
            SchemaMode::Disabled => false,
            SchemaMode::Manual => false,
            SchemaMode::Derived => true,
        }
    }

    fn use_in_crd(self) -> bool {
        match self {
            SchemaMode::Disabled => false,
            SchemaMode::Manual => true,
            SchemaMode::Derived => true,
        }
    }
}

impl FromMeta for SchemaMode {
    fn from_string(value: &str) -> darling::Result<Self> {
        match value {
            "disabled" => Ok(SchemaMode::Disabled),
            "manual" => Ok(SchemaMode::Manual),
            "derived" => Ok(SchemaMode::Derived),
            x => Err(darling::Error::unknown_value(x)),
        }
    }
}

pub(crate) fn derive(input: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let derive_input: DeriveInput = match syn::parse2(input) {
        Err(err) => return err.to_compile_error(),
        Ok(di) => di,
    };
    // Limit derive to structs
    match derive_input.data {
        Data::Struct(_) | Data::Enum(_) => {}
        _ => {
            return syn::Error::new_spanned(
                &derive_input.ident,
                r#"Unions can not #[derive(CustomResource)]"#,
            )
            .to_compile_error()
        }
    }
    let kube_attrs = match KubeAttrs::from_derive_input(&derive_input) {
        Err(err) => return err.write_errors(),
        Ok(attrs) => attrs,
    };

    let KubeAttrs {
        group,
        kind,
        kind_struct,
        version,
        namespaced,
        derives,
        schema: schema_mode,
        status,
        plural,
        singular,
        categories,
        shortnames,
        printcolums,
        scale,
        crates:
            Crates {
                kube_core,
                k8s_openapi,
                schemars,
                serde,
                serde_json,
                std,
            },
    } = kube_attrs;

    let struct_name = kind_struct.unwrap_or_else(|| kind.clone());
    if derive_input.ident == struct_name {
        return syn::Error::new_spanned(
            derive_input.ident,
            r#"#[derive(CustomResource)] `kind = "..."` must not equal the struct name (this is generated)"#,
        )
        .to_compile_error();
    }
    let visibility = derive_input.vis;
    let ident = derive_input.ident;

    // 1. Create root object Foo and truncate name from FooSpec

    // Default visibility is `pub(crate)`
    // Default generics is no generics (makes little sense to re-use CRD kind?)
    // We enforce metadata + spec's existence (always there)
    // => No default impl
    let rootident = Ident::new(&struct_name, Span::call_site());
    let rootident_str = rootident.to_string();

    // if status set, also add that
    let StatusInformation {
        field: status_field,
        default: status_default,
        impl_hasstatus,
    } = process_status(&rootident, &status, &visibility, &kube_core);
    let has_status = status.is_some();
    let serialize_status = if has_status {
        quote! {
            if let Some(status) = &self.status {
                obj.serialize_field("status", &status)?;
            }
        }
    } else {
        quote! {}
    };
    let has_status_value = if has_status {
        quote! { self.status.is_some() }
    } else {
        quote! { false }
    };

    let mut derive_paths: Vec<Path> = vec![
        syn::parse_quote! { #serde::Deserialize },
        syn::parse_quote! { Clone },
        syn::parse_quote! { Debug },
    ];
    let mut has_default = false;
    for d in &derives {
        if d == "Default" {
            has_default = true; // overridden manually to avoid confusion
        } else {
            match syn::parse_str(d) {
                Err(err) => return err.to_compile_error(),
                Ok(d) => derive_paths.push(d),
            }
        }
    }

    // Enable schema generation by default as in v1 it is mandatory.
    let schema_mode = schema_mode.unwrap_or(SchemaMode::Derived);
    // We exclude fields `apiVersion`, `kind`, and `metadata` from our schema because
    // these are validated by the API server implicitly. Also, we can't generate the
    // schema for `metadata` (`ObjectMeta`) because it doesn't implement `JsonSchema`.
    let schemars_skip = if schema_mode.derive() {
        quote! { #[schemars(skip)] }
    } else {
        quote! {}
    };
    if schema_mode.derive() {
        derive_paths.push(syn::parse_quote! { #schemars::JsonSchema });
    }

    let docstr = format!(" Auto-generated derived type for {} via `CustomResource`", ident);
    let root_obj = quote! {
        #[doc = #docstr]
        #[automatically_derived]
        #[allow(missing_docs)]
        #[derive(#(#derive_paths),*)]
        #[serde(rename_all = "camelCase")]
        #visibility struct #rootident {
            #schemars_skip
            #visibility metadata: #k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta,
            #visibility spec: #ident,
            #status_field
        }
        impl #rootident {
            /// Spec based constructor for derived custom resource
            pub fn new(name: &str, spec: #ident) -> Self {
                Self {
                    metadata: #k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                        name: Some(name.to_string()),
                        ..Default::default()
                    },
                    spec: spec,
                    #status_default
                }
            }
        }
        impl #serde::Serialize for #rootident {
            fn serialize<S: #serde::Serializer>(&self, ser: S) -> #std::result::Result<S::Ok, S::Error> {
                use #serde::ser::SerializeStruct;
                let mut obj = ser.serialize_struct(#rootident_str, 4 + usize::from(#has_status_value))?;
                obj.serialize_field("apiVersion", &<#rootident as #kube_core::Resource>::api_version(&()))?;
                obj.serialize_field("kind", &<#rootident as #kube_core::Resource>::kind(&()))?;
                obj.serialize_field("metadata", &self.metadata)?;
                obj.serialize_field("spec", &self.spec)?;
                #serialize_status
                obj.end()
            }
        }
    };

    // 2. Implement Resource trait
    let name = singular.unwrap_or_else(|| kind.to_ascii_lowercase());
    let plural = plural.unwrap_or_else(|| to_plural(&name));
    let (scope, scope_quote) = if namespaced {
        ("Namespaced", quote! { #kube_core::NamespaceResourceScope })
    } else {
        ("Cluster", quote! { #kube_core::ClusterResourceScope })
    };

    let api_ver = format!("{}/{}", group, version);
    let impl_resource = quote! {
        impl #kube_core::Resource for #rootident {
            type DynamicType = ();
            type Scope = #scope_quote;

            fn group(_: &()) -> std::borrow::Cow<'_, str> {
               #group.into()
            }

            fn kind(_: &()) -> std::borrow::Cow<'_, str> {
                #kind.into()
            }

            fn version(_: &()) -> std::borrow::Cow<'_, str> {
                #version.into()
            }

            fn api_version(_: &()) -> std::borrow::Cow<'_, str> {
                #api_ver.into()
            }

            fn plural(_: &()) -> std::borrow::Cow<'_, str> {
                #plural.into()
            }

            fn meta(&self) -> &#k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                &self.metadata
            }

            fn meta_mut(&mut self) -> &mut #k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                &mut self.metadata
            }
        }
    };

    // 3. Implement Default if requested
    let impl_default = if has_default {
        quote! {
            impl Default for #rootident {
                fn default() -> Self {
                    Self {
                        metadata: #k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta::default(),
                        spec: Default::default(),
                        #status_default
                    }
                }
            }
        }
    } else {
        quote! {}
    };

    // 4. Implement CustomResource

    // Compute a bunch of crd props
    let printers = format!("[ {} ]", printcolums.join(",")); // hacksss
    let scale_code = if let Some(s) = scale { s } else { "".to_string() };

    // Ensure it generates for the correct CRD version (only v1 supported now)
    let apiext = quote! {
        #k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1
    };
    let extver = quote! {
        #kube_core::crd::v1
    };

    let shortnames_slice = {
        let names = shortnames
            .iter()
            .map(|name| quote! { #name, })
            .collect::<TokenStream>();
        quote! { &[#names] }
    };

    let categories_json = serde_json::to_string(&categories).unwrap();
    let short_json = serde_json::to_string(&shortnames).unwrap();
    let crd_meta_name = format!("{}.{}", plural, group);
    let crd_meta = quote! { { "name": #crd_meta_name } };

    let schemagen = if schema_mode.use_in_crd() {
        quote! {
            // Don't use definitions and don't include `$schema` because these are not allowed.
            let gen = #schemars::gen::SchemaSettings::openapi3()
                .with(|s| {
                    s.inline_subschemas = true;
                    s.meta_schema = None;
                })
                .with_visitor(#kube_core::schema::StructuralSchemaRewriter)
                .into_generator();
            let schema = gen.into_root_schema_for::<Self>();
        }
    } else {
        // we could issue a compile time warning for this, but it would hit EVERY compile, which would be noisy
        // eprintln!("warning: kube-derive configured with manual schema generation");
        // users must manually set a valid schema in crd.spec.versions[*].schema - see examples: crd_derive_no_schema
        quote! {
            let schema: Option<#k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::JSONSchemaProps> = None;
        }
    };

    let jsondata = quote! {
        #schemagen

        let jsondata = #serde_json::json!({
            "metadata": #crd_meta,
            "spec": {
                "group": #group,
                "scope": #scope,
                "names": {
                    "categories": categories,
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
    };

    // Implement the CustomResourceExt trait to allow users writing generic logic on top of them
    let impl_crd = quote! {
        impl #extver::CustomResourceExt for #rootident {

            fn crd() -> #apiext::CustomResourceDefinition {
                let columns : Vec<#apiext::CustomResourceColumnDefinition> = #serde_json::from_str(#printers).expect("valid printer column json");
                let scale: Option<#apiext::CustomResourceSubresourceScale> = if #scale_code.is_empty() {
                    None
                } else {
                    #serde_json::from_str(#scale_code).expect("valid scale subresource json")
                };
                let categories: Vec<String> = #serde_json::from_str(#categories_json).expect("valid categories");
                let shorts : Vec<String> = #serde_json::from_str(#short_json).expect("valid shortnames");
                let subres = if #has_status {
                    if let Some(s) = &scale {
                        #serde_json::json!({
                            "status": {},
                            "scale": scale
                        })
                    } else {
                        #serde_json::json!({"status": {} })
                    }
                } else {
                    #serde_json::json!({})
                };

                #jsondata
                #serde_json::from_value(jsondata)
                    .expect("valid custom resource from #[kube(attrs..)]")
            }

            fn crd_name() -> &'static str {
                #crd_meta_name
            }

            fn api_resource() -> #kube_core::dynamic::ApiResource {
                #kube_core::dynamic::ApiResource::erase::<Self>(&())
            }

            fn shortnames() -> &'static [&'static str] {
                #shortnames_slice
            }
        }
    };

    let impl_hasspec = generate_hasspec(&ident, &rootident, &kube_core);

    // Concat output
    quote! {
        #root_obj
        #impl_resource
        #impl_default
        #impl_crd
        #impl_hasspec
        #impl_hasstatus
    }
}

/// This generates the code for the `#kube_core::object::HasSpec` trait implementation.
///
/// All CRDs have a spec so it is implemented for all of them.
///
/// # Arguments
///
/// * `ident`: The identity (name) of the spec struct
/// * `root ident`: The identity (name) of the main CRD struct (the one we generate in this macro)
/// * `kube_core`: The path stream for the analagous kube::core import location from users POV
fn generate_hasspec(spec_ident: &Ident, root_ident: &Ident, kube_core: &Path) -> TokenStream {
    quote! {
        impl #kube_core::object::HasSpec for #root_ident {
            type Spec = #spec_ident;

            fn spec(&self) -> &#spec_ident {
                &self.spec
            }

            fn spec_mut(&mut self) -> &mut #spec_ident {
                &mut self.spec
            }
        }
    }
}

struct StatusInformation {
    /// The code to be used for the field in the main struct
    field: TokenStream,
    /// The initialization code to use in a `Default` and `::new()` implementation
    default: TokenStream,
    /// The implementation code for the `HasStatus` trait
    impl_hasstatus: TokenStream,
}

/// This processes the `status` field of a CRD.
///
/// As it is optional some features will be turned on or off depending on whether it's available or not.
///
/// # Arguments
///
/// * `root ident`: The identity (name) of the main CRD struct (the one we generate in this macro)
/// * `status`: The optional name of the `status` struct to use
/// * `visibility`: Desired visibility of the generated field
/// * `kube_core`: The path stream for the analagous kube::core import location from users POV
///
/// returns: A `StatusInformation` struct
fn process_status(
    root_ident: &Ident,
    status: &Option<String>,
    visibility: &Visibility,
    kube_core: &Path,
) -> StatusInformation {
    if let Some(status_name) = &status {
        let ident = format_ident!("{}", status_name);
        StatusInformation {
            field: quote! {
                #[serde(skip_serializing_if = "Option::is_none")]
                #visibility status: Option<#ident>,
            },
            default: quote! { status: None, },
            impl_hasstatus: quote! {
                impl #kube_core::object::HasStatus for #root_ident {

                    type Status = #ident;

                    fn status(&self) -> Option<&#ident> {
                        self.status.as_ref()
                    }

                    fn status_mut(&mut self) -> &mut Option<#ident> {
                        &mut self.status
                    }
                }
            },
        }
    } else {
        let empty_quote = quote! {};
        StatusInformation {
            field: empty_quote.clone(),
            default: empty_quote.clone(),
            impl_hasstatus: empty_quote,
        }
    }
}

// Simple pluralizer.
// Duplicating the code from kube (without special casing) because it's simple enough.
// Irregular plurals must be explicitly specified.
fn to_plural(word: &str) -> String {
    // Words ending in s, x, z, ch, sh will be pluralized with -es (eg. foxes).
    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        return format!("{}es", word);
    }

    // Words ending in y that are preceded by a consonant will be pluralized by
    // replacing y with -ies (eg. puppies).
    if word.ends_with('y') {
        if let Some(c) = word.chars().nth(word.len() - 2) {
            if !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') {
                // Remove 'y' and add `ies`
                let mut chars = word.chars();
                chars.next_back();
                return format!("{}ies", chars.as_str());
            }
        }
    }

    // All other words will have "s" added to the end (eg. days).
    format!("{}s", word)
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO Unit test `derive`

    #[test]
    fn test_parse_default() {
        let input = quote! {
            #[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
            #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
            struct FooSpec { foo: String }
        };
        let input = syn::parse2(input).unwrap();
        let kube_attrs = KubeAttrs::from_derive_input(&input).unwrap();
        assert_eq!(kube_attrs.group, "clux.dev".to_string());
        assert_eq!(kube_attrs.version, "v1".to_string());
        assert_eq!(kube_attrs.kind, "Foo".to_string());
        assert!(kube_attrs.namespaced);
    }
}
