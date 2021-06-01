//! Middleware for customizing client.

mod auth;
mod base_uri;
mod headers;

pub(crate) use self::{
    auth::{AuthLayer, Authentication},
    headers::set_default_headers,
};
pub use base_uri::{SetBaseUri, SetBaseUriLayer};
