//! API helpers

/// Empty struct for when no Spec is required
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Void {}

mod reflector;
pub use self::reflector::Reflector;

mod informer;
pub use self::informer::Informer;

mod raw;
pub use raw::{DeleteParams, ListParams, PatchParams, PatchStrategy, PostParams, PropagationPolicy, RawApi};

mod typed;
pub use typed::Api;

mod subresource;
pub use subresource::{LogParams, Scale, ScaleSpec, ScaleStatus};

mod resource;
pub use self::resource::{KubeObject, Object, ObjectList, WatchEvent};

mod openapi;
#[cfg(feature = "openapi")] mod snowflake;
#[cfg(feature = "openapi")] pub use snowflake::{v1ConfigMap, v1Event, v1Secret};

mod metadata;
pub use self::metadata::{Initializers, ListMeta, ObjectMeta, OwnerReference, TypeMeta};
