//! API helpers

/// Empty struct for when no Spec is required
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](https://docs.rs/serde/1.0.104/serde/trait.Deserialize.html) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct NotUsed {}

/// Use [`NotUsed`](notused.struct.html) instead. Renamed to avoid confusion with [`void::Void`](https://docs.rs/void/1.0.2/void/enum.Void.html).
#[deprecated]
pub type Void = NotUsed;

pub(crate) mod raw;
pub use raw::{DeleteParams, ListParams, PatchParams, PatchStrategy, PostParams, PropagationPolicy, RawApi};

pub(crate) mod typed;
pub use typed::Api;

mod subresource;
pub use subresource::{LogParams, Scale, ScaleSpec, ScaleStatus};

pub(crate) mod resource;
pub use self::resource::{KubeObject, Object, ObjectList, WatchEvent};

mod openapi;
#[cfg(feature = "openapi")] mod snowflake;
#[cfg(feature = "openapi")] pub use snowflake::{v1ConfigMap, v1Event, v1Secret};

mod metadata;
pub use self::metadata::{Initializers, ListMeta, ObjectMeta, OwnerReference, TypeMeta};
