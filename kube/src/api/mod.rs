//! API helpers for structured interaction with the Kubernetes API

use serde::{Deserialize, Serialize};

/// Empty struct for when data should be discarded
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](serde::Deserialize) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct NotUsed {}

pub(crate) use kube_core::params;
use kube_core::request;
pub use params::{
    DeleteParams, ListParams, Patch, PatchParams, PostParams, Preconditions, PropagationPolicy,
};
pub use request::Request;

pub(crate) mod typed;
pub use typed::Api;

mod gvk;
pub use gvk::{GroupVersionKind, GroupVersionResource};

mod dynamic;
pub use dynamic::{ApiResource, DynamicObject};

#[cfg(feature = "ws")] mod remote_command;
#[cfg(feature = "ws")] pub use remote_command::AttachedProcess;

mod subresource;
#[cfg(feature = "ws")]
pub use subresource::{AttachParams, Attachable, Executable};
pub use subresource::{EvictParams, Evictable, LogParams, Loggable, ScaleSpec, ScaleStatus};

pub(crate) mod object;
pub use self::object::{Object, ObjectList, WatchEvent};

mod metadata;
pub use self::metadata::{ListMeta, ObjectMeta, Resource, ResourceExt, TypeMeta};

#[cfg(feature = "admission")] pub mod admission;
