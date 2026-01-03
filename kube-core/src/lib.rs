//! Types and traits necessary for interacting with the Kubernetes API
//!
//! This crate provides the minimal apimachinery necessary to make requests to the kubernetes API.
//!
//! It does not export export a client, but it also has almost no dependencies.
//!
//! Everything in this crate is re-exported from [`kube`](https://crates.io/crates/kube)
//! (even with zero features) under [`kube::core`]((https://docs.rs/kube/*/kube/core/index.html)).
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
#[cfg(feature = "admission")]
pub mod admission;

pub mod conversion;

pub mod discovery;

pub mod duration;
pub use duration::Duration;

pub mod dynamic;
pub use dynamic::{ApiResource, DynamicObject};

pub mod crd;
pub use crd::CustomResourceExt;

pub mod cel;
pub use cel::{ListMerge, MapMerge, Message, Reason, Rule, StructMerge};

#[cfg(feature = "schema")]
pub use cel::{merge_properties, merge_strategy, merge_strategy_property, validate, validate_property};

pub mod gvk;
pub use gvk::{GroupVersion, GroupVersionKind, GroupVersionResource};

pub mod metadata;
pub use metadata::{ListMeta, ObjectMeta, PartialObjectMeta, PartialObjectMetaExt, TypeMeta};

pub mod labels;

#[cfg(feature = "kubelet-debug")] pub mod kubelet_debug;

pub mod object;
pub use object::{NotUsed, Object, ObjectList};

pub mod params;

pub mod request;
pub use request::Request;

mod resource;
pub use resource::{
    ClusterResourceScope, DynamicResourceScope, NamespaceResourceScope, Resource, ResourceExt, ResourceScope,
    SubResourceScope, api_version_from_group_version,
};

pub mod response;
pub use response::Status;

pub use labels::{Expression, ParseExpressionError, Selector, SelectorExt};

#[cfg_attr(docsrs, doc(cfg(feature = "schema")))]
#[cfg(feature = "schema")]
pub mod schema;

pub mod subresource;

pub mod util;

pub mod watch;
pub use watch::WatchEvent;

mod version;
pub use version::Version;

pub mod error_boundary;
pub use error_boundary::DeserializeGuard;

/// This type used to be a payload for `kube::Error::Api`. It has been replaced by `Status`.
/// In the interest of backward compatibility, we keep this type definition here as a deprecated alias.
/// `Status` better reflects the Kubernetes API conventions and is more versatile.
/// If you need to migrate your code, simply replace `ErrorResponse` with `Status` and also check
/// helper methods implemented on `Status` for easily identifying common error cases.
///
/// As a reference below is the original definition of this type:
///
/// An error response from the API.
/// #[derive(Error, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
/// #[error("{message}: {reason}")]
/// pub struct ErrorResponse {
///     /// The status
///     pub status: String,
///     /// A message about the error
///     #[serde(default)]
///     pub message: String,
///     /// The reason for the error
///     #[serde(default)]
///     pub reason: String,
///     /// The error code
///     pub code: u16,
/// }
///
#[deprecated(
    since = "3.0.0",
    note = "use kube::core::Status instead. \n\nNote that the Error::Api constructor based pattern matches have been helper functions due to boxing.\n  See Status docs for the new pattern:\n  https://docs.rs/kube/latest/kube/core/struct.Status.html"
)]
pub type ErrorResponse = Status;
