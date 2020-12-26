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
/// This will generate a **root object** containing your spec and metadata.
/// This root object will implement the [`k8s_openapi::Metadata`] + [`k8s_openapi::Resource`]
/// traits so it can be used with [`kube::Api`].
///
/// The generated type will also implement a `::crd` method to generate the crd
/// at the specified api version (or `v1` if unspecified).
///
/// # Example
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use k8s_openapi::Resource;
/// use kube_derive::CustomResource;
/// use schemars::JsonSchema;
///
/// #[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
/// #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
/// struct FooSpec {
///     info: String,
/// }
///
/// println!("kind = {}", Foo::KIND); // impl k8s_openapi::Resource
/// let f = Foo::new("foo-1", FooSpec {
///     info: "informative info".into(),
/// });
/// println!("foo: {:?}", f); // debug print on generated type
/// println!("crd: {}", serde_yaml::to_string(&Foo::crd()).unwrap()); // crd yaml
/// ```
///
/// This example creates a `struct Foo` containing metadata, the spec,
/// and optionally status. The **generated** type `Foo` can be used with the [`kube`] crate
/// as an `Api<Foo>` object (`FooSpec` can not be used with [`Api`][`kube::Api`]).
///
/// ```rust,ignore
///  let client = Client::try_default().await?;
///  let foos: Api<Foo> = Api::namespaced(client.clone(), "default");
///
///  let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
///  crds.patch("foos.clux.dev", &ssapply, serde_yaml::to_vec(&Foo::crd())?).await
///  ```
///
/// This example posts the generated `::crd` to the `CustomResourceDefinition` API.
/// After this has been accepted (few secs max), you can start using `foos` as a normal
/// kube `Api` object. See the `crd_` prefixed [examples](https://github.com/clux/kube-rs/blob/master/examples/)
/// for details on this.
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
/// ### `#[kube(apiextensions = "v1beta1")]`
/// The version for `CustomResourceDefinition` desired in the `apiextensions.k8s.io` group.
/// Default is `v1` (for clusters >= 1.17). If using kubernetes <= 1.16 pluase use `v1beta1`.
///
/// **NOTE**: Support for `v1` requires deriving the openapi v3 `JsonSchema` via the `schemars` dependency.
///
/// ### `#[kube(namespaced)]`
/// To specify that this is a namespaced resource rather than cluster level.
///
/// ### `#[kube(struct = "StructName")]`
/// Customize the name of the generated root struct (defaults to `kind`).
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
/// ### `#[kube(printcolumn = r#"json"#)]`
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
/// use schemars::JsonSchema;
///
/// #[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
/// #[kube(
///     group = "clux.dev",
///     version = "v1",
///     kind = "Foo",
///     struct = "FooCrd",
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
/// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
/// struct FooStatus {
///     replicas: i32
/// }
/// ```
///
/// ## Generated code
///
/// The example above will roughly generate:
/// ```ignore
/// #[derive(Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
/// #[serde(rename_all = "camelCase")]
/// pub struct FooCrd {
///     api_version: String,
///     kind: String,
///     metadata: ObjectMeta,
///     spec: FooSpec,
///     status: Option<FooStatus>,
/// }
/// impl k8s_openapi::Resource for FooCrd {...}
/// impl k8s_openapi::Metadata for FooCrd {...}
///
/// impl FooCrd {
///     pub fn new(name: &str, spec: FooSpec) -> Self { ... }
///     pub fn crd() -> k8s_openapi::...::CustomResourceDefinition { ... }
/// }
/// ```
///
/// ## Debugging
/// Try `cargo-expand` to see your own macro expansion.
///
/// ## Installation
/// Enable the `derive` feature on the `kube` crate:
///
/// ```toml
/// kube = { version = "...", features = ["derive"] }
/// ```
///
/// ## Runtime dependencies
/// Due to [rust-lang/rust#54363](https://github.com/rust-lang/rust/issues/54363), we cannot be resilient against crate renames within our generated code.
/// It's therefore **required** that you have the following crates in scope, not renamed:
///
/// - `serde_json`
/// - `k8s_openapi`
/// - `schemars` (by default, unless `schema` feature disabled)
///
/// You are ultimately responsible for maintaining the versions and feature flags of these libraries.
///
/// [`kube`]: https://docs.rs/kube
/// [`kube::Api`]: https://docs.rs/kube/*/kube/struct.Api.html
/// [`k8s_openapi::Metadata`]: https://docs.rs/k8s-openapi/*/k8s_openapi/trait.Metadata.html
/// [`k8s_openapi::Resource`]: https://docs.rs/k8s-openapi/*/k8s_openapi/trait.Resource.html
#[proc_macro_derive(CustomResource, attributes(kube))]
pub fn derive_custom_resource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_custom_derive::<CustomResource>(input)
}
