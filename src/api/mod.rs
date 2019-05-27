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

mod api;
pub use api::{
    Api,
    ListParams,
    PostParams,
    DeleteParams,
    PropagationPolicy,
};

mod resource;
pub use self::resource::{
    Object,
    ObjectList,
    WatchEvent,
    ApiError,
    //PostResponse,
    //CreateResponse,
    //Response,
};

mod metadata;
pub use self::metadata::{
    Metadata,
    Initializers,
};
