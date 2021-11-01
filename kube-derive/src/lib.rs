//! A crate for kube's derive macros.
#![deny(unsafe_code)]
#![deny(missing_docs)]
#![recursion_limit = "1024"]
extern crate proc_macro;
#[macro_use] extern crate quote;

mod custom_resource;

/// A custom derive for kubernetes custom resource definitions.
///
/// This will generate a **root object** containing your spec and metadata.
/// This root object will implement the [`kube::Resource`] trait
/// so it can be used with [`kube::Api`].
///
/// The generated type will also implement kube's [`kube::CustomResourceExt`] trait to generate the crd
/// and generate [`kube::core::ApiResource`] information for use with the dynamic api.
///
/// # Example
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use kube::core::{Resource, CustomResourceExt};
/// use kube_derive::CustomResource;
/// use schemars::JsonSchema;
///
/// #[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
/// #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
/// struct FooSpec {
///     info: String,
/// }
///
/// println!("kind = {}", Foo::kind(&())); // impl kube::Resource
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
/// kube `Api` object. See the `crd_` prefixed [examples](https://github.com/kube-rs/kube-rs/blob/master/examples/)
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
/// Default is `v1` (for clusters >= 1.17). If using kubernetes <= 1.16 please use `v1beta1`.
///
/// - **NOTE**: Support for `v1` requires deriving the openapi v3 `JsonSchema` via the `schemars` dependency.
/// - **NOTE**: When using `v1beta` the associated `CustomResourceExt` trait lives in `kube::core::crd::v1beta`
///
/// ### `#[kube(singular = "nonstandard-singular")]`
/// To specify the singular name. Defaults to lowercased `kind`.
///
/// ### `#[kube(plural = "nonstandard-plural")]`
/// To specify the plural name. Defaults to inferring from singular.
///
/// ### `#[kube(namespaced)]`
/// To specify that this is a namespaced resource rather than cluster level.
///
/// ### `#[kube(struct = "StructName")]`
/// Customize the name of the generated root struct (defaults to `kind`).
///
/// ### `#[kube(kube_core = "::kube::core")]`
/// Customize the crate name the generated code will reach into (defaults to `::kube::core`).
/// Should be one of `kube::core`, `kube_client::core` or `kube_core`.
///
/// ### `#[kube(k8s_openapi = "::k8s_openapi")]`
/// Customize the crate name the generated code will use for [`k8s_openapi`](https://docs.rs/k8s-openapi/) (defaults to `::k8s_openapi`).
///
/// ### `#[kube(schemars = "::schemars")]`
/// Customize the crate name the generated code will use for [`schemars`](https://docs.rs/schemars/) (defaults to `::schemars`).
///
/// ### `#[kube(serde = "::serde")]`
/// Customize the crate name the generated code will use for [`serde`](https://docs.rs/serde/) (defaults to `::serde`).
///
/// ### `#[kube(serde_json = "::serde_json")]`
/// Customize the crate name the generated code will use for [`serde_json`](https://docs.rs/serde_json/) (defaults to `::serde_json`).
///
/// ### `#[kube(status = "StatusStructName")]`
/// Adds a status struct to the top level generated type and enables the status
/// subresource in your crd.
///
/// ### `#[kube(derive = "Trait")]`
/// Adding `#[kube(derive = "PartialEq")]` is required if you want your generated
/// top level type to be able to `#[derive(PartialEq)]`
///
/// ### `#[kube(derive_schema = false)]`
/// Disables the automatic `#[derive(JsonSchema)]` on the top level generated type.
///
/// This can be used to provide a completely custom schema, or to interact with third-party custom resources,
/// where you are not responsible for installing the `CustomResourceDefinition`.
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
/// use validator::Validate;
///
/// #[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, Validate, JsonSchema)]
/// #[kube(
///     group = "clux.dev",
///     version = "v1",
///     kind = "Foo",
///     struct = "FooCrd",
///     namespaced,
///     status = "FooStatus",
///     derive = "PartialEq",
///     singular = "foot",
///     plural = "feetz",
///     shortname = "f",
///     scale = r#"{"specReplicasPath":".spec.replicas", "statusReplicasPath":".status.replicas"}"#,
///     printcolumn = r#"{"name":"Spec", "type":"string", "description":"name of foo", "jsonPath":".spec.name"}"#
/// )]
/// #[serde(rename_all = "camelCase")]
/// struct FooSpec {
///     #[validate(length(min = 3))]
///     data: String,
///     replicas_count: i32
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
/// impl kube::Resource for FooCrd {...}
///
/// impl FooCrd {
///     pub fn new(name: &str, spec: FooSpec) -> Self { ... }
///     pub fn crd() -> k8s_openapi::...::CustomResourceDefinition { ... }
/// }
/// ```
///
/// ## Customizing Schemas
/// Should you need to customize the schemas, you can use:
/// - [Serde/Schemars Attributes](https://graham.cool/schemars/examples/3-schemars_attrs/) (no need to duplicate serde renames)
/// - [`#[schemars(schema_with = "func")]`](https://graham.cool/schemars/examples/7-custom_serialization/) (e.g. like in the [`crd_derive` example](https://github.com/kube-rs/kube-rs/blob/master/examples/crd_derive.rs))
/// - `impl JsonSchema` on a type / newtype around external type. See [#129](https://github.com/kube-rs/kube-rs/issues/129#issuecomment-750852916)
/// - [`#[validate(...)]` field attributes with validator](https://github.com/Keats/validator) for kubebuilder style validation rules (see [`crd_api` example](https://github.com/kube-rs/kube-rs/blob/master/examples/crd_api.rs)))
///
/// You might need to override parts of the schemas (for fields in question) when you are:
/// - **using complex enums**: enums do not currently generate [structural schemas](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#specifying-a-structural-schema), so kubernetes won't support them by default
/// - **customizing [merge-strategies](https://kubernetes.io/docs/reference/using-api/server-side-apply/#merge-strategy)** (e.g. like in the [`crd_derive_schema` example](https://github.com/kube-rs/kube-rs/blob/master/examples/crd_derive_schema.rs))
///
/// See [kubernetes openapi validation](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definitions/#validation) for the format of the OpenAPI v3 schemas.
///
/// If you have to override a lot, [you can opt-out of schema-generation entirely](#kubederive_schema--false)
///
/// ## Advanced Features
/// - **embedding k8s-openapi types** can be done by enabling the `schemars` feature of `k8s-openapi` from [`0.13.0`](https://github.com/Arnavion/k8s-openapi/blob/master/CHANGELOG.md#v0130-2021-08-09)
/// - **adding validation** via [validator crate](https://github.com/Keats/validator) is supported from `schemars` >= [`0.8.5`](https://github.com/GREsau/schemars/blob/master/CHANGELOG.md#085---2021-09-20)
///
/// ### Validation Caveats
/// The supported **`#[validate]` attrs also exist as `#[schemars]` attrs** so you can use those directly if you do not require the validation to run client-side (in your code).
/// Otherwise, you should `#[derive(Validate)]` on your struct to have both server-side (kubernetes) and client-side validation.
///
/// When using `validator` directly, you must add it to your dependencies (with the `derive` feature).
///
/// Make sure your validation rules are static and handled by `schemars`:
/// - validations from `#[validate(custom = "some_fn")]` will not show up in the schema.
/// - similarly; [nested / must_match / credit_card were unhandled by schemars at time of writing](https://github.com/GREsau/schemars/pull/78)
///
/// For sanity, you should review the generated schema before sending it to kubernetes.
///
/// ## Versioning
/// Note that any changes to your struct / validation rules / serialization attributes will require you to re-apply the generated
/// schema to kubernetes, so that the apiserver can validate against the right version of your structs.
///
/// How to best deal with version changes has not been fully sketched out. See [#569](https://github.com/kube-rs/kube-rs/issues/569).
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
/// [`kube::Resource`]: https://docs.rs/kube/*/kube/trait.Resource.html
/// [`kube::core::ApiResource`]: https://docs.rs/kube/*/kube/core/struct.ApiResource.html
/// [`kube::CustomResourceExt`]: https://docs.rs/kube/*/kube/trait.CustomResourceExt.html
#[proc_macro_derive(CustomResource, attributes(kube))]
pub fn derive_custom_resource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    custom_resource::derive(proc_macro2::TokenStream::from(input)).into()
}
