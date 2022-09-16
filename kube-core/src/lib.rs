//! Crate with types and traits necessary for interacting with the Kubernetes API
//!
//! This crate provides the minimal apimachinery necessary to make requests to the kubernetes API.
//!
//! It does not export export a client, but it also has almost no dependencies.
//!
//! Everything in this crate is re-exported from [`kube`](https://crates.io/crates/kube)
//! (even with zero features) under [`kube::core`]((https://docs.rs/kube/*/kube/core/index.html)).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

#[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
#[cfg(feature = "admission")]
pub mod admission;

pub mod conversion;

pub mod discovery;

pub mod dynamic;
pub use dynamic::{ApiResource, DynamicObject};

pub mod crd;
pub use crd::CustomResourceExt;

pub mod gvk;
pub use gvk::{GroupVersion, GroupVersionKind, GroupVersionResource};

pub mod metadata;
pub use metadata::{ListMeta, ObjectMeta, TypeMeta};

pub mod object;
pub use object::{NotUsed, Object, ObjectList};

pub mod params;

pub mod request;
pub use request::Request;

mod resource;
pub use resource::{
    ClusterResourceScope, DynamicResourceScope, NamespaceResourceScope, Resource, ResourceExt, ResourceScope,
    SubResourceScope,
};

pub mod response;
pub use response::Status;

#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
#[cfg(feature = "schema")]
pub mod schema;

pub mod subresource;

pub mod util;

pub mod watch;
pub use watch::WatchEvent;

mod error;
pub use error::ErrorResponse;

mod version;
pub use version::Version;
