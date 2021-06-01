//! Middleware for customizing client.

mod auth;
mod base_uri;
mod headers;
mod log;

pub(crate) use self::{
    auth::{AuthLayer, Authentication},
    headers::set_default_headers,
    log::LogRequest,
};
pub use base_uri::{SetBaseUri, SetBaseUriLayer};
