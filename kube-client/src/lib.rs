//! Crate for interacting with the Kubernetes API
//!
//! This crate includes the tools for manipulating Kubernetes resources as
//! well as keeping track of those resources as they change over time
//!
//! # Example
//!
//! The following example will create a [`Pod`](k8s_openapi::api::core::v1::Pod)
//! and then watch for it to become available using a manual [`Api::watch`] call.
//!
//! ```rust,no_run
//! use futures::{StreamExt, TryStreamExt};
//! use kube_client::api::{Api, ResourceExt, ListParams, PatchParams, Patch};
//! use kube_client::Client;
//! use k8s_openapi::api::core::v1::Pod;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Read the environment to find config for kube client.
//!     // Note that this tries an in-cluster configuration first,
//!     // then falls back on a kubeconfig file.
//!     let client = Client::try_default().await?;
//!
//!     // Interact with pods in the configured namespace with the typed interface from k8s-openapi
//!     let pods: Api<Pod> = Api::default_namespaced(client);
//!
//!     // Create a Pod (cheating here with json, but it has to validate against the type):
//!     let patch: Pod = serde_json::from_value(serde_json::json!({
//!         "apiVersion": "v1",
//!         "kind": "Pod",
//!         "metadata": {
//!             "name": "my-pod"
//!         },
//!         "spec": {
//!             "containers": [
//!                 {
//!                     "name": "my-container",
//!                     "image": "myregistry.azurecr.io/hello-world:v1",
//!                 },
//!             ],
//!         }
//!     }))?;
//!
//!     // Apply the Pod via server-side apply
//!     let params = PatchParams::apply("myapp");
//!     let result = pods.patch("my-pod", &params, &Patch::Apply(&patch)).await?;
//!
//!     // List pods in the configured namespace
//!     for p in pods.list(&ListParams::default()).await? {
//!         println!("found pod {}", p.name());
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! For more details, see:
//!
//! - [`Client`](crate::client) for the extensible Kubernetes client
//! - [`Config`](crate::config) for the Kubernetes config abstraction
//! - [`Api`](crate::Api) for the generic api methods available on Kubernetes resources
//! - [k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/) for how to create typed kubernetes objects directly
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
    pub mod api;
    pub mod discovery;
    pub mod client;

    #[doc(inline)]
    pub use api::Api;
    #[doc(inline)]
    pub use client::Client;
    #[doc(inline)]
    pub use discovery::Discovery;
}

cfg_config! {
    pub mod config;
    #[doc(inline)]
    pub use config::Config;
}

cfg_error! {
    pub mod error;
    #[doc(inline)] pub use error::Error;
    /// Convient alias for `Result<T, Error>`
    pub type Result<T, E = Error> = std::result::Result<T, E>;
}

pub use crate::core::{CustomResourceExt, Resource, ResourceExt};
/// Re-exports from kube_core
pub use kube_core as core;
