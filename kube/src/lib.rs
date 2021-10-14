//! Umbrella crate for working with kubernetes
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(unsafe_code)]

macro_rules! cfg_client {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "client")))]
            #[cfg(feature = "client")]
            $item
        )*
    }
}
macro_rules! cfg_config {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "config")))]
            #[cfg(feature = "config")]
            $item
        )*
    }
}

macro_rules! cfg_error {
    ($($item:item)*) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(any(feature = "config", feature = "client"))))]
            #[cfg(any(feature = "config", feature = "client"))]
            $item
        )*
    }
}

cfg_client! {
    pub use kube_client::api;
    pub use kube_client::discovery;
    pub use kube_client::client;

    #[doc(inline)]
    pub use api::Api;
    #[doc(inline)]
    pub use client::Client;
    #[doc(inline)]
    pub use discovery::Discovery;
}

cfg_config! {
    pub use kube_client::config;
    #[doc(inline)]
    pub use config::Config;
}

cfg_error! {
    pub use kube_client::error;
    #[doc(inline)] pub use error::Error;
    /// Convient alias for `Result<T, Error>`
    pub type Result<T, E = Error> = std::result::Result<T, E>;
}

/// Re-exports from kube-derive
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use kube_derive::CustomResource;

/// Re-exports from kube-runtime
#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
pub use kube_runtime as runtime;

/// Re-exports from kube_core crate.
pub mod core {
    #[cfg(feature = "admission")]
    #[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
    pub use kube_core::admission;
    pub use kube_core::{
        crd::{self, CustomResourceExt},
        dynamic::{self, ApiResource, DynamicObject},
        gvk::{self, GroupVersionKind, GroupVersionResource},
        metadata::{self, ListMeta, ObjectMeta, TypeMeta},
        object::{self, NotUsed, Object, ObjectList},
        request::{self, Request},
        response::{self, Status},
        watch::{self, WatchEvent},
        Resource, ResourceExt,
    };
}
pub use crate::core::{CustomResourceExt, Resource, ResourceExt};
