//! API helpers

/// Empty struct for when no Spec is required
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](https://docs.rs/serde/1.0.104/serde/trait.Deserialize.html) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct NotUsed {}

pub(crate) mod raw;
pub use raw::{DeleteParams, ListParams, PatchParams, PatchStrategy, PostParams, PropagationPolicy, RawApi};

//pub(crate) mod typed;
//pub use typed::Api;

//mod subresource;
//pub use subresource::{LoggingObject, LogParams, Scale, ScaleSpec, ScaleStatus};
//#[cfg(feature = "openapi")]
//impl LoggingObject for k8s_openapi::api::core::v1::Pod {}



pub(crate) mod resource;
pub use self::resource::{KubeObject, Object, ObjectList, WatchEvent};

mod metadata;
pub use self::metadata::{Initializers, ListMeta, ObjectMeta, OwnerReference, TypeMeta};
