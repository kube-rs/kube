#[cfg(feature = "admission")] pub mod admission;

pub mod api_resource;
pub use api_resource::ApiResource;
pub mod dynamic;
pub use dynamic::DynamicObject;

pub mod gvk;
pub use gvk::{GroupVersionKind, GroupVersionResource};

pub mod metadata;

pub mod object;
pub use object::WatchEvent;

pub mod params;

pub mod request;
pub use request::Request;

mod resource;
pub use resource::{Resource, ResourceExt};

pub mod response;

pub mod subresource;

#[macro_use] extern crate log;


mod error;
pub use error::{Error, ErrorResponse};
pub type Result<T, E = Error> = std::result::Result<T, E>;
