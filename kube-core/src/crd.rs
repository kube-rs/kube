//! Traits and tyes for CustomResources

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions as apiexts;

/// Types for v1 CustomResourceDefinitions
pub mod v1 {
    use super::apiexts;
    use crate::discovery::ApiResource;
    /// Extension trait that will be implemented by kube-derive
    ///
    /// This trait variant is implemented by default (or when `#[kube(apiextensions = "v1")]`)
    pub trait CustomResourceExt {
        /// Helper to generate the CRD including the JsonSchema
        ///
        /// This is using the stable v1::CustomResourceDefinitions (present in kubernetes >= 1.16)
        fn crd() -> apiexts::v1::CustomResourceDefinition;
        /// Helper to generate the api information type for use with the dynamic `Api`
        fn api_resource() -> ApiResource;
    }
}

/// Types for legacy v1beta1 CustomResourceDefinitions
pub mod v1beta1 {
    use super::apiexts;
    use crate::discovery::ApiResource;
    /// Extension trait that will be implemented by kube-derive for legacy v1beta1::CustomResourceDefinitions
    ///
    /// This trait variant is only implemented with `#[kube(apiextensions = "v1beta1")]`
    pub trait CustomResourceExt {
        /// Helper to generate the legacy CRD without a JsonSchema
        ///
        /// This is using v1beta1::CustomResourceDefinitions (which will be removed in kubernetes 1.22)
        fn crd() -> apiexts::v1beta1::CustomResourceDefinition;
        /// Helper to generate the api information type for use with the dynamic `Api`
        fn api_resource() -> ApiResource;
    }
}

/// re-export the current latest version until a newer one is available in cloud providers
pub use v1::CustomResourceExt;
