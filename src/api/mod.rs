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
    DeleteParams,
    PropagationPolicy,
};

mod typed;
pub use typed::Api;

mod resource;
pub use self::resource::{
    Object,
    ObjectList,
    WatchEvent,
    //PostResponse,
    //CreateResponse,
    //Response,
};

mod metadata;
pub use self::metadata::{
    Metadata,
    Initializers,
};
