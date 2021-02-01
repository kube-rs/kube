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
//! use kube::api::{Api, Meta, ListParams, PostParams, WatchEvent};
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
//!             WatchEvent::Added(o) => println!("Added {}", Meta::name(&o)),
//!             WatchEvent::Modified(o) => {
//!                 let s = o.status.as_ref().expect("status exists on pod");
//!                 let phase = s.phase.clone().unwrap_or_default();
//!                 println!("Modified: {} with phase: {}", Meta::name(&o), phase);
//!             }
//!             WatchEvent::Deleted(o) => println!("Deleted {}", Meta::name(&o)),
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

#[macro_use] extern crate static_assertions;
assert_cfg!(
    all(
        not(all(feature = "native-tls", feature = "rustls-tls")),
        any(feature = "native-tls", feature = "rustls-tls")
    ),
    "Must use exactly one of native-tls or rustls-tls features"
);

#[macro_use] extern crate log;

pub mod api;
pub mod client;
pub mod config;
#[deprecated(note = "Replaced by the kube-runtime crate", since = "0.38.0")]
// Rust doesn't allow items within a deprecated module to interact with each other..
#[allow(deprecated)]
pub mod runtime;

pub mod error;
mod oauth2;

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use kube_derive::CustomResource;

pub use api::{Api, DynamicResource, Resource};
#[doc(inline)] pub use client::Client;
#[doc(inline)] pub use config::Config;
#[doc(inline)] pub use error::Error;

/// Convient alias for `Result<T, Error>`
pub type Result<T, E = Error> = std::result::Result<T, E>;
