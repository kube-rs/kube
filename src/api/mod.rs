//! API helpers

/// Empty struct for when no Spec is required
#[derive(Clone, Deserialize, Default)]
pub struct Void {}

mod reflector;
pub use self::reflector::{
    Cache,
    Reflector,
};

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
    PatchStrategy
};

mod typed;
pub use typed::{
    Api,
    // well, ok:
    Scale,
    ScaleSpec,
    ScaleStatus,
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
pub use snowflake::Event;

mod metadata;
pub use self::metadata::{
    ObjectMeta,
    TypeMeta,
    Initializers,
};
