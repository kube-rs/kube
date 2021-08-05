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
//! use kube::api::{Api, ResourceExt, ListParams, PostParams, WatchEvent};
//! use kube::Client;
//! use k8s_openapi::api::core::v1::Pod;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), kube::Error> {
//!     // Read the environment to find config for kube client.
//!     // Note that this tries an in-cluster configuration first,
//!     // then falls back on a kubeconfig file.
//!     let client = Client::try_default().await?;
//!
//!     // Get a strongly typed handle to the Kubernetes API for interacting
//!     // with pods in the "default" namespace.
//!     let pods: Api<Pod> = Api::namespaced(client, "default");
//!
//!     // Create a pod from JSON
//!     let pod = serde_json::from_value(serde_json::json!({
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
//!     // Create the pod
//!     let pod = pods.create(&PostParams::default(), &pod).await?;
//!
//!     // Start a watch call for pods matching our name
//!     let lp = ListParams::default()
//!             .fields(&format!("metadata.name={}", "my-pod"))
//!             .timeout(10);
//!     let mut stream = pods.watch(&lp, "0").await?.boxed();
//!
//!     // Observe the pods phase for 10 seconds
//!     while let Some(status) = stream.try_next().await? {
//!         match status {
//!             WatchEvent::Added(o) => println!("Added {}", o.name()),
//!             WatchEvent::Modified(o) => {
//!                 let s = o.status.as_ref().expect("status exists on pod");
//!                 let phase = s.phase.clone().unwrap_or_default();
//!                 println!("Modified: {} with phase: {}", o.name(), phase);
//!             }
//!             WatchEvent::Deleted(o) => println!("Deleted {}", o.name()),
//!             WatchEvent::Error(e) => println!("Error {}", e),
//!             _ => {}
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

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

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use kube_derive::CustomResource;

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
