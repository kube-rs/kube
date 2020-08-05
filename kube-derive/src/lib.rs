//! A crate for kube's derive macros.
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![recursion_limit = "1024"]
extern crate proc_macro;
#[macro_use] extern crate quote;
use proc_macro::TokenStream;
use syn::Result;

trait CustomDerive: Sized {
    fn parse(input: syn::DeriveInput, tokens: proc_macro2::TokenStream) -> Result<Self>;
    fn emit(self) -> Result<proc_macro2::TokenStream>;
}

fn run_custom_derive<T>(input: TokenStream) -> TokenStream
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
    fn spanning(self, spanned: impl quote::ToTokens) -> Result<T>;
}

impl<T, E> ResultExt<T> for std::result::Result<T, E>
where
    E: std::fmt::Display,
{
    fn spanning(self, spanned: impl quote::ToTokens) -> Result<T> {
        self.map_err(|err| syn::Error::new_spanned(spanned, err))
    }
}

// #[derive(CustomResource)]
mod custom_resource;
use custom_resource::CustomResource;

/// A custom derive for kubernetes custom resource definitions.
///
/// This will generate a root object that implements the `k8s_openapi::Metadata` and
/// `k8s_openapi::Resource` traits for this type so it can be used with `kube::Api`
///
/// Additionally, it will implement a `Foo::crd` function which will generate the,
/// CustomResourceDefinition at the specified api version (or v1 if unspecified).
///
/// # Example
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use kube_derive::CustomResource;
///
/// #[derive(CustomResource, Clone, Debug, Deserialize, Serialize)]
/// #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
/// struct FooSpec {
///     info: String,
/// }
///
/// fn main() {
///     use k8s_openapi::Resource;
///     println!("kind = {}", Foo::KIND); // impl k8s_openapi::Resource
///     let f = Foo::new("foo-1", FooSpec {
///         info: "informative info".into(),
///     });
///     println!("foo: {:?}", f); // debug print on generated type
///     println!("crd: {}", serde_yaml::to_string(&Foo::crd()).unwrap()); // crd yaml
/// }
/// ```
///
/// This example creates a `struct Foo` containing metadata, the spec,
/// and optionally status. The generated type `Foo` can be used with the `kube` crate
/// as an `Api<Foo>` object.
///
/// ## Required properties
///
/// ### `#[kube(group = "mygroup.tld")]`
/// Your cr api group. The part before the slash in the top level `apiVersion` key.
///
/// ### `#[kube(version = "v1")]`
/// Your cr api version. The part after the slash in the top level `apiVersion` key.
///
/// ### `#[kube(kind = "Kind")]`
/// Name of your kind and your generated root type.
///
/// ## Optional `#[kube]` attributes
///
/// ### `#[kube(namespaced)]`
/// To specify that this is a namespaced resource rather than cluster level.
///
/// ### `#[kube(status = "StatusStructName")]`
/// Adds a status struct to the top level generated type and enables the status
/// subresource in your crd.
///
/// ### `#[kube(derive = "Trait")]`
/// Adding `#[kube(derive = "PartialEq")]` is required if you want your generated
/// top level type to be able to `#[derive(PartialEq)]`
///
/// ### `#[kube(scale = r#"json"#)]`
/// Allow customizing the scale struct for the [scale subresource](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#subresources).
///
/// ### `#[kube(printcoloum = r#"json"#)]`
/// Allows adding straight json to [printcolumns](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#additional-printer-columns).
///
/// ### `#[kube(shortname = "sn")]`
/// Add a single shortname to the generated crd.
///
/// ## Example with all properties
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use kube_derive::CustomResource;
///
/// #[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone)]
/// #[kube(
///     group = "clux.dev",
///     version = "v1",
///     kind = "Foo",
///     namespaced,
///     status = "FooStatus",
///     derive = "PartialEq",
///     shortname = "f",
///     scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#,
///     printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
/// )]
/// struct FooSpec {
///     data: String,
///     replicas: i32
/// }
///
/// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
/// struct FooStatus {
///     replicas: i32
/// }
/// ```
///
/// ## Generated code
///
/// The example above will roughly generate:
/// ```ignore
/// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
/// #[serde(rename_all = "camelCase")]
/// pub struct Foo {
///     api_version: String,
///     kind: String,
///     metadata: ObjectMeta,
///     spec: FooSpec,
///     status: Option<FooStatus>,
/// }
/// impl k8s_openapi::Resource for Foo {...}
/// impl k8s_openapi::Metadata for Foo {...}
///
/// impl Foo {
///     pub fn new(name: &str, spec: FooSpec) -> Self { ... }
///     pub fn crd() -> k8s_openapi::...::CustomResourceDefinition { ... }
/// }
/// ```
///
/// And the `Foo::crd` will contribute to the largest amount of generated code.
///
/// ## Debugging
/// Try `cargo-expand` to see your own macro expansion.
#[proc_macro_derive(CustomResource, attributes(kube))]
pub fn derive_custom_resource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_custom_derive::<CustomResource>(input)
}
