//! API helpers

/// Empty struct for when no Spec is required
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct Void {}

mod reflector;
pub use self::reflector::Reflector;

mod informer;
pub use self::informer::{
    Informer,
};

mod raw;
pub use raw::{
    RawApi,
    ListParams,
    PostParams,
    PatchParams,
    DeleteParams,
    PropagationPolicy,
    PatchStrategy,
    LogParams
};

mod typed;
pub use typed::{
    Api,
    // well, ok:
    Scale,
    ScaleSpec,
    ScaleStatus,
    Log
};

mod resource;
pub use self::resource::{
    Object,
    ObjectList,
    WatchEvent,
    KubeObject,
};

#[cfg(feature = "openapi")]
mod openapi;
#[cfg(feature = "openapi")]
mod snowflake;
#[cfg(feature = "openapi")]
pub use snowflake::{v1Event, v1Secret, v1ConfigMap};

mod metadata;
pub use self::metadata::{
    ObjectMeta,
    TypeMeta,
    Initializers,
};
