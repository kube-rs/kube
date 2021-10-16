//! Traits and tyes for CustomResources

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions as apiexts;

/// Types for v1 CustomResourceDefinitions
pub mod v1 {
    /// Extension trait that is implemented by kube-derive
    ///
    /// This trait variant is implemented by default (or when `#[kube(apiextensions = "v1")]`)
    pub trait CustomResourceExt {
        /// Helper to generate the CRD including the JsonSchema
        ///
        /// This is using the stable v1::CustomResourceDefinitions (present in kubernetes >= 1.16)
        fn crd() -> super::apiexts::v1::CustomResourceDefinition;
        /// Helper to return the name of this `CustomResourceDefinition` in kubernetes.
        ///
        /// This is not the name of an _instance_ of this custom resource but the `CustomResourceDefinition` object itself.
        fn crd_name() -> &'static str;
        /// Helper to generate the api information type for use with the dynamic `Api`
        fn api_resource() -> crate::discovery::ApiResource;
        /// Shortnames of this resource type.
        ///
        /// For example: [`Pod`] has the shortname alias `po`.
        ///
        /// NOTE: This function returns *declared* short names (at compile-time, using the `#[kube(shortname = "foo")]`), not the
        /// shortnames registered with the Kubernetes API (which is what tools such as `kubectl` look at).
        ///
        /// [`Pod`]: `k8s_openapi::api::core::v1::Pod`
        fn shortnames() -> &'static [&'static str];
    }
}

/// Types for legacy v1beta1 CustomResourceDefinitions
#[cfg(feature = "deprecated-crd-v1beta1")]
pub mod v1beta1 {
    /// Extension trait that is implemented by kube-derive for legacy v1beta1::CustomResourceDefinitions
    ///
    /// This trait variant is only implemented with `#[kube(apiextensions = "v1beta1")]`
    pub trait CustomResourceExt {
        /// Helper to generate the legacy CRD without a JsonSchema
        ///
        /// This is using v1beta1::CustomResourceDefinitions (which will be removed in kubernetes 1.22)
        fn crd() -> super::apiexts::v1beta1::CustomResourceDefinition;
        /// Helper to return the name of this `CustomResourceDefinition` in kubernetes.
        ///
        /// This is not the name of an _instance_ of this custom resource but the `CustomResourceDefinition` object itself.
        fn crd_name() -> &'static str;
        /// Helper to generate the api information type for use with the dynamic `Api`
        fn api_resource() -> crate::discovery::ApiResource;
        /// Shortnames of this resource type.
        ///
        /// For example: [`Pod`] has the shortname alias `po`.
        ///
        /// NOTE: This function returns *declared* short names (at compile-time, using the `#[kube(shortname = "foo")]`), not the
        /// shortnames registered with the Kubernetes API (which is what tools such as `kubectl` look at).
        ///
        /// [`Pod`]: `k8s_openapi::api::core::v1::Pod`
        fn shortnames() -> &'static [&'static str];
    }
}

/// re-export the current latest version until a newer one is available in cloud providers
pub use v1::CustomResourceExt;
