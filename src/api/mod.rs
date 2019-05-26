//! API helpers to make use of k8s-openapi easier

/// Empty struct for Void
#[derive(Clone, Deserialize, Default)]
pub struct Void {}
// Shortcut type for discarding one type parameter option
//pub type Void = Option<Discard>;

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
