//! Crate for interacting with the Kubernetes API
//!
//! This crate includes the tools for manipulating Kubernetes resources as
//! well as keeping track of those resources as they change over time
//!
//! # Example
//!
//! The following example will crate a [`Pod`][k8s_openapi::api::core::v1::Pod]
//! and then watch for it to become available
//!
//! ```rust,no_run
//! use k8s_openapi::api::core::v1::Pod;
//! use tokio::stream::StreamExt as _;
//! use futures_util::stream::StreamExt as _;
//!
//! async {
//!     // Read the environment to find config for kube client.
//!     // Note that this tries an in-cluster configuration first,
//!     // then falls back on a kubeconfig file.
//!     let kube_client = kube::Client::try_default()
//!        .await
//!        .expect("kubeconfig failed to load");
//!     
//!     // Get a strongly typed handle to the Kubernetes API for interacting
//!     // with pods in the "default" namespace.
//!     let pods: kube::Api<Pod> = kube::Api::namespaced(kube_client.clone(), "default");
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
//!     })).unwrap();
//!
//!     // Create the pod
//!     let pod = pods.create(&kube::api::PostParams::default(), &pod).await.unwrap();
//!
//!     // Create an informer for watching events about
//!     let informer: kube::runtime::Informer<Pod> = kube::runtime::Informer::new(
//!         kube_client,
//!         kube::api::ListParams::default()
//!             .fields("metadata.name=my-container")
//!             .timeout(10),
//!         kube::Resource::namespaced::<Pod>("default"),
//!     );
//!
//!     // Get an event stream from the informer
//!     let mut events_stream = informer.poll().await.unwrap().boxed();
//!     
//!     // Keep getting events from the events stream
//!     while let Some(event) = events_stream.try_next().await.unwrap() {
//!         use kube::api::WatchEvent;
//!         match event {
//!             WatchEvent::Modified(e) if e.status.as_ref().unwrap().phase.as_ref().unwrap() == "Running" => {
//!                 println!("It's running!");
//!             }
//!             WatchEvent::Error(e) => {
//!                 panic!("WatchEvent error: {:?}", e);
//!             }
//!             _ => {}
//!         }
//!     }
//! };
//! ```

#![deny(missing_docs)]

#[macro_use] extern crate log;

pub mod api;
pub mod client;
pub mod config;
pub mod runtime;

pub mod error;
mod oauth2;

pub use api::{Api, Resource};
#[doc(inline)] pub use client::Client;
#[doc(inline)] pub use config::Config;
#[doc(inline)] pub use error::Error;

/// Convient alias for `Result<T, Error>`
pub type Result<T> = std::result::Result<T, Error>;
