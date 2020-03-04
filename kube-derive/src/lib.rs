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
/// ```rust,ignore
/// #[derive(CustomResource, Clone, Debug, PartialEq, Deserialize, Serialize)]
/// #[kube(group = "clux.dev", version = "v1", namespaced)]
/// struct FooSpec {
///     prop1: String,
///     prop2: Vec<bool>,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     prop3: Option<i32>,
/// }
/// ```
///
/// This example creates a `struct Foo` containing metadata, the spec,
/// and optionally status.
///
/// The struct should be named MyKindSpec for it to infer the Kind is MyKind normally.
/// But you can also use an arbitrary name if you supply `#[kube(kind = "MyFoo")]`
///
/// Setting printercolumns + subresources are also supported.
///
///
/// Try `cargo-expand` to see your own macro expansion.
#[proc_macro_derive(CustomResource, attributes(kube))]
pub fn derive_custom_resource(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    run_custom_derive::<CustomResource>(input)
}
