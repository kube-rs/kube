#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;

/// ApiError for when things fail
///
/// This can be parsed into as an error handling fallback.
/// Replacement data for reqwest::Response::error_for_status,
/// which is often lacking in good permission errors.
/// It's also used in `WatchEvent` from watch calls.
///
/// It's quite commont to get a `410 Gone` when the resourceVersion is too old.
#[derive(Fail, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[fail(display = "ApiError {} ({:?})", reason, message)]
pub struct ApiError {
    pub status: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub reason: String,
    pub code: u16,
}

#[derive(Debug, Fail)]
pub enum ErrorKind {
    /// The main error type when most things are working
    #[fail(display = "{}", _0)]
    Api(#[fail(cause)] ApiError),

    /// Common error case when requesting parsing into own structs
    #[fail(display = "Error deserializing response")]
    SerdeParse,

    #[fail(display = "Error building request")]
    RequestBuild,
    #[fail(display = "Error executing request")]
    RequestSend,
    #[fail(display = "Error parsing response")]
    RequestParse,
    #[fail(display = "Invalid API method {}", _0)]
    InvalidMethod(String),
    #[fail(display = "Request validation failed with {}", _0)]
    RequestValidation(String),
}

use std::fmt::{self, Display};
use failure::{Context, Fail, Backtrace};

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }
    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}
impl Error {
    pub fn kind(&self) -> &ErrorKind {
        self.inner.get_context()
    }
    pub fn api_error(&self) -> Option<ApiError> {
        match self.kind() {
            ErrorKind::Api(e) => Some(e.clone()),
            _ => None,
        }
    }
}
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { inner: Context::new(kind) }
    }
}
impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}

/*
#[derive(Fail, Debug)]
pub enum Error {
    #[fail(display = "{}", _0)]
    Api(#[fail(cause)] ApiError),

    #[fail(display = "{}", _0)]
    Other(#[fail(cause)] failure::Error)
}
*/
pub type Result<T> = std::result::Result<T, Error>;



pub mod client;
pub mod config;
pub mod api;
