//! API helpers

/// Empty struct for when no Spec is required
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](https://docs.rs/serde/1.0.104/serde/trait.Deserialize.html) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct NotUsed {}

pub(crate) mod resource;
pub use resource::{
    DeleteParams, ListParams, PatchParams, PatchStrategy, PostParams, PropagationPolicy, Resource,
};

pub(crate) mod typed;
pub use typed::Api;

mod crds;
pub use crds::{CrBuilder, CustomResource};

mod subresource;
pub use subresource::{LogParams, LoggingObject, Scale, ScaleSpec, ScaleStatus};

pub(crate) mod object;
pub use self::object::{Object, ObjectList, WatchEvent};

mod metadata;
pub use self::metadata::{ListMeta, MetaContent, Metadata, ObjectMeta, TypeMeta};
