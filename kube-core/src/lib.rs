use thiserror::Error;

pub mod api_resource;
pub mod gvk;
pub mod params;
pub mod request;
pub mod resource;
pub mod subresource;

#[macro_use] extern crate log;

#[derive(Error, Debug)]
pub enum Error {
    /// A request validation failed
    #[error("Request validation failed with {0}")]
    RequestValidation(String),

    /// Common error case when requesting parsing into own structs
    #[error("Error deserializing response")]
    SerdeError(#[from] serde_json::Error),

    /// Http based error
    #[error("HttpError: {0}")]
    HttpError(#[from] http::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
