pub mod api_resource;
pub mod dynamic;
pub mod gvk;
pub mod metadata;
pub mod object;
pub mod params;
pub mod request;
pub mod resource;
pub mod subresource;

#[macro_use] extern crate log;


mod error;
pub use error::{Error, ErrorResponse};
pub type Result<T, E = Error> = std::result::Result<T, E>;
