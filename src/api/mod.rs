//! API helpers to make use of k8s-openapi easier

/// Shortcut type for discarding one type parameter option
pub type Void = Option<()>;

mod reflector;
pub use self::reflector::{
    ResourceMap,
    Reflector,
};

mod informer;
pub use self::informer::{
    Informer,
};

mod resource;
pub use self::resource::{
    Resource,
    ApiResource,
    ResourceType,
    QueryParams,
    WatchEvent,
    ApiError,
};

mod metadata;
pub use self::metadata::{
    Metadata,
    Initializers,
};
