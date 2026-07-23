use darling::{
    FromDeriveInput, FromMeta,
    util::{Override, parse_expr},
};
use syn::{Expr, Meta, Path, parse_quote};

mod kvp;
mod printcolumn;
mod scale;

pub use kvp::KeyValuePair;
pub use printcolumn::PrintColumn;
pub use scale::Scale;

/// Values we can parse from #[kube(attrs)]
#[derive(Debug, FromDeriveInput)]
#[darling(attributes(kube))]
pub struct KubeAttrs {
    pub group: String,
    pub version: String,
    pub kind: String,
    pub doc: Option<String>,
    #[darling(rename = "root")]
    pub kind_struct: Option<String>,
    /// lowercase plural of kind (inferred if omitted)
    pub plural: Option<String>,
    /// singular defaults to lowercased kind
    pub singular: Option<String>,
    #[darling(default)]
    pub namespaced: bool,
    #[darling(multiple, rename = "derive")]
    pub derives: Vec<String>,
    #[darling(multiple, rename = "attr")]
    pub attributes: Vec<KubeRootMeta>,
    pub schema: Option<SchemaMode>,
    pub status: Option<Path>,
    #[darling(multiple, rename = "category")]
    pub categories: Vec<String>,
    #[darling(multiple, rename = "shortname")]
    pub shortnames: Vec<String>,

    /// Add additional print columns, see [Kubernetes docs][1].
    ///
    /// [1]: https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#additional-printer-columns
    #[darling(multiple, rename = "printcolumn")]
    pub printcolumns: Vec<PrintColumn>,
    #[darling(multiple)]
    pub selectable: Vec<String>,

    /// Customize the scale subresource, see [Kubernetes docs][1].
    ///
    /// [1]: https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#scale-subresource
    pub scale: Option<Scale>,

    #[darling(default)]
    pub crates: Crates,
    #[darling(multiple, rename = "annotation")]
    pub annotations: Vec<KeyValuePair>,
    #[darling(multiple, rename = "label")]
    pub labels: Vec<KeyValuePair>,
    #[darling(multiple, rename = "validation", with = parse_expr::preserve_str_literal)]
    pub validations: Vec<Expr>,

    /// Generate client-side CEL validation methods (`validate_cel` / `validate_cel_update`).
    ///
    /// Requires the downstream crate to enable the `kube/cel` feature, since the generated
    /// code references `kube::core::cel::*`.
    #[darling(default)]
    pub cel: bool,

    /// Sets the `storage` property to `true` or `false`.
    ///
    /// Defaults to `true`.
    #[darling(default = Self::default_storage_arg)]
    pub storage: bool,

    /// Sets the `served` property to `true` or `false`.
    ///
    /// Defaults to `true`.
    #[darling(default = Self::default_served_arg)]
    pub served: bool,

    /// Sets the `deprecated` and optionally the `deprecationWarning` property.
    ///
    /// See https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-deprecation
    pub deprecated: Option<Override<String>>,
}

impl KubeAttrs {
    fn default_storage_arg() -> bool {
        // This defaults to true to be backwards compatible.
        true
    }

    fn default_served_arg() -> bool {
        // This defaults to true to be backwards compatible.
        true
    }
}

#[derive(Debug, FromMeta)]
pub struct Crates {
    #[darling(default = "Self::default_kube")]
    pub kube: Path,
    #[darling(default = "Self::default_kube_core")]
    pub kube_core: Path,
    #[darling(default = "Self::default_k8s_openapi")]
    pub k8s_openapi: Path,
    #[darling(default = "Self::default_schemars")]
    pub schemars: Path,
    #[darling(default = "Self::default_serde")]
    pub serde: Path,
    #[darling(default = "Self::default_serde_json")]
    pub serde_json: Path,
    #[darling(default = "Self::default_std")]
    pub std: Path,
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

    fn default_kube() -> Path {
        parse_quote! { ::kube }
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

#[derive(Debug, PartialEq, Eq, Clone, Copy, FromMeta)]
pub enum SchemaMode {
    Disabled,
    Manual,
    Derived,
}

impl SchemaMode {
    pub fn derive(self) -> bool {
        match self {
            SchemaMode::Disabled => false,
            SchemaMode::Manual => false,
            SchemaMode::Derived => true,
        }
    }

    pub fn use_in_crd(self) -> bool {
        match self {
            SchemaMode::Disabled => false,
            SchemaMode::Manual => true,
            SchemaMode::Derived => true,
        }
    }
}

/// Attribute meta that should be added to the root of the custom resource.
/// Wrapper around `Meta` to implement custom validation logic for `darling`.
/// The validation rejects attributes for `derive`, `serde` and `schemars`.
/// For `derive` there is `#[kube(derive=...)]` which does specialized handling
/// and for `serde` and `schemars` allowing to set attributes could result in conflicts
/// or unexpected behaviour with respect to other parts of the generated code.
#[derive(Debug)]
pub struct KubeRootMeta(pub Meta);

impl FromMeta for KubeRootMeta {
    fn from_string(value: &str) -> darling::Result<Self> {
        /// Attributes that are not allowed to be set via `#[kube(attr=...)]`.
        const NOT_ALLOWED_ATTRIBUTES: [&str; 3] = ["derive", "serde", "schemars"];

        let meta = syn::parse_str::<Meta>(value)?;
        if let Some(ident) = meta.path().get_ident()
            && NOT_ALLOWED_ATTRIBUTES.iter().any(|el| ident == el)
        {
            if ident == "derive" {
                return Err(darling::Error::custom(
                    r#"#[derive(CustomResource)] `kube(attr = "...")` does not support to set derives, you likely want to use `kube(derive = "...")`."#,
                ));
            }
            return Err(darling::Error::custom(format!(
                r#"#[derive(CustomResource)] `kube(attr = "...")` does not support to set the attributes {NOT_ALLOWED_ATTRIBUTES:?} as they might lead to unexpected behaviour.`"#,
            )));
        }

        Ok(Self(meta))
    }
}
