//! Middleware for customizing client.

mod auth;
mod base_uri;
#[cfg(feature = "gzip")] mod compression;
mod headers;
mod log;

#[cfg(feature = "gzip")]
pub(crate) use self::compression::{accept_compressed, maybe_decompress};
pub(crate) use self::{
    auth::{AuthLayer, Authentication},
    headers::set_default_headers,
    log::LogRequest,
};
pub use base_uri::{SetBaseUri, SetBaseUriLayer};
