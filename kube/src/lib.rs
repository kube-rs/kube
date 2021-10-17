//! Kube is un umbrella-crate for interacting with [Kubernetes](http://kubernetes.io) in rust.
//!
//! # Overview
//!
//! Kube-rs contains a kubernetes client, a controller runtime, a derive macro, and various tooling
//! required for building applications or controllers that interact with kubernetes.
//!
//! The main exports are:
//!
//! - [`client`](crate::client) module with the kubernetes `Client`
//! - [`api`](crate::api) module with the generic kubernetes `Api`
//! - [`derive`](crate::CustomResource) module with the `CustomResource` derive for building controllers types
//! - [`runtime`](crate::runtime) module with a `Controller` / `watcher` / `reflector` / `store`
//! - [`core`](crate::core) module with generics from `apimachinery`
//!
//! You can use each of these as you need with the help of the [exported features](https://github.com/kube-rs/kube-rs/blob/master/kube/Cargo.toml#L18).
//!
//! # Using the Client
//! ```no_run
//! use futures::{StreamExt, TryStreamExt};
//! use kube::{Client, api::{Api, ResourceExt, ListParams, PostParams}};
//! use k8s_openapi::api::core::v1::Pod;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), kube::Error> {
//!     // Infer the runtime environment and try to create a kubernetes Client
//!     let client = Client::try_default().await?;
//!
//!     // Read pods in the configured namespace into the typed interface from k8s-openapi
//!     let pods: Api<Pod> = Api::default_namespaced(client);
//!     for p in pods.list(&ListParams::default()).await? {
//!         println!("found pod {}", p.name());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! For details, see:
//!
//! - [`Client`](crate::client) for the extensible kubernetes client
//! - [`Api`](crate::Api) for the generic api methods available on kubernetes resources
//! - [k8s-openapi](https://docs.rs/k8s-openapi/*/k8s_openapi/) for documentation about the generated kubernetes types
//!
//! # Using the Runtime with the Derive macro
//!
//! ```no_run
//! use schemars::JsonSchema;
//! use serde::{Deserialize, Serialize};
//! use serde_json::json;
//! use validator::Validate;
//! use futures::{StreamExt, TryStreamExt};
//! use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
//! use kube::{
//!     api::{Api, DeleteParams, ListParams, PostParams, ResourceExt},
//!     core::CustomResourceExt,
//!     Client, CustomResource,
//!     runtime::{watcher, utils::try_flatten_applied},
//! };
//!
//! // Our custom resource
//! #[derive(CustomResource, Deserialize, Serialize, Clone, Debug, Validate, JsonSchema)]
//! #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
//! pub struct FooSpec {
//!     info: String,
//!     #[validate(length(min = 3))]
//!     name: String,
//!     replicas: i32,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::try_default().await?;
//!     let crds: Api<CustomResourceDefinition> = Api::all(client.clone());
//!
//!     // Create the CRD so users can apply Foo instances in kubernetes
//!     let foocrd = Foo::crd();
//!     crds.create(&PostParams::default(), &foocrd).await?;
//!
//!     // Wait for the api to catch up (can do this more precisely - see #655)
//!     tokio::time::sleep(std::time::Duration::from_secs(1)).await;
//!
//!     // Manage the Foo CR
//!     let foos: Api<Foo> = Api::default_namespaced(client.clone());
//!
//!     // Watch for apply events to all foos
//!     let lp = ListParams::default();
//!     let mut apply_stream = try_flatten_applied(watcher(foos, lp)).boxed();
//!     while let Some(f) = apply_stream.try_next().await? {
//!         println!("saw apply to {}", f.name());
//!     }
//!     Ok(())
//! }
//! ```
//!
//! For details, see:
//!
//! - [`CustomResource`](crate::CustomResource) for documentation how to configure custom resources
//! - [`runtime::watcher`](crate::runtime::watcher()) for how to long-running watches work and why you want to use this over `Api::watch`
//! - [`runtime`](crate::runtime) for abstractions that help with more complicated kubernetes application
//!
//! # Examples
//! A large list of complete, runnable examples with explainations are available in the [examples folder](https://github.com/kube-rs/kube-rs/tree/master/examples).
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

/// Re-exports from [`kube-derive`](kube_derive)
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use kube_derive::CustomResource;

/// Re-exports from [`kube-runtime`](kube_runtime)
#[cfg(feature = "runtime")]
#[cfg_attr(docsrs, doc(cfg(feature = "runtime")))]
#[doc(inline)]
pub use kube_runtime as runtime;

pub use crate::core::{CustomResourceExt, Resource, ResourceExt};
/// Re-exports from [`kube_core`](kube_core)
#[doc(inline)]
pub use kube_core as core;
