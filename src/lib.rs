use thiserror::Error;

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;

#[derive(Error, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[error("{message}: {reason}")]
pub struct ErrorResponse {
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub reason: String,
    pub code: u16,
}

#[derive(Error, Debug)]
pub enum Error {
    /// ApiError for when things fail
    ///
    /// This can be parsed into as an error handling fallback.
    /// Replacement data for reqwest::Response::error_for_status,
    /// which is often lacking in good permission errors.
    /// It's also used in `WatchEvent` from watch calls.
    ///
    /// It's quite commont to get a `410 Gone` when the resourceVersion is too old.
    #[error("ApiError: {0} ({0:?})")]
    Api(ErrorResponse),

    // Request errors
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("HttpError: {0}")]
    HttpError(#[from] http::Error),

    /// Common error case when requesting parsing into own structs
    #[error("Error deserializing response")]
    SerdeError(#[from] serde_json::Error),

    #[error("Error building request")]
    RequestBuild,
    #[error("Error executing request")]
    RequestSend,
    #[error("Error parsing response")]
    RequestParse,
    #[error("Invalid API method {0}")]
    InvalidMethod(String),
    #[error("Request validation failed with {0}")]
    RequestValidation(String),

    /// Configuration error
    #[error("Error loading kube config: {0}")]
    KubeConfig(String),

    #[error("SslError: {0}")]
    SslError(String),
}

pub type Result<T> = std::result::Result<T, Error>;

pub mod api;
pub mod client;
pub mod config;
mod oauth2;
// pub mod runtime;
