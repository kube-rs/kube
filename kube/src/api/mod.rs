//! API helpers for structured interaction with the Kubernetes API

use serde::{Deserialize, Serialize};

/// Empty struct for when data should be discarded
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](https://docs.rs/serde/1.0.104/serde/trait.Deserialize.html) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct NotUsed {}

pub(crate) mod params;
pub use params::{DeleteParams, ListParams, PatchParams, PatchStrategy, PostParams, PropagationPolicy};
mod resource;
pub use resource::Resource;

pub(crate) mod typed;
pub use typed::Api;

mod dynamic;
pub use dynamic::DynamicResource;

mod subresource;
pub use subresource::{LogParams, LoggingObject, ScaleSpec, ScaleStatus};

pub(crate) mod object;
pub use self::object::{Object, ObjectList, WatchEvent};

mod metadata;
pub use self::metadata::{ListMeta, Meta, ObjectMeta, TypeMeta};
