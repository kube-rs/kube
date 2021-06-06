//! Crate with types and traits necessary for interacting with the Kubernetes API
//!
//! This crate is available as a minimal alternative to `kube` where a client is not available.
//! The same information here is always re-exported from `kube` under `kube::core`.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(unsafe_code)]

#[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
#[cfg(feature = "admission")]
pub mod admission;

pub mod discovery;

pub mod dynamic;
pub use dynamic::DynamicObject;

pub mod crd;
pub use crd::CustomResourceExt;

pub mod gvk;
pub use gvk::{GroupVersion, GroupVersionKind, GroupVersionResource};

pub mod metadata;

pub mod object;

pub mod params;

pub mod request;
pub use request::Request;

mod resource;
pub use resource::{Resource, ResourceExt};

pub mod response;

pub mod subresource;

pub mod watch;
pub use watch::WatchEvent;

mod error;
pub use error::{Error, ErrorResponse};

/// Convient alias for `Result<T, Error>`
pub type Result<T, E = Error> = std::result::Result<T, E>;
