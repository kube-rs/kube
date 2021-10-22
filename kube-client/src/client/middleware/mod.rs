//! Middleware types returned from `ConfigExt` methods.
use tower::{util::Either, Layer};
pub(crate) use tower_http::auth::AddAuthorizationLayer;

mod base_uri;
mod refresh_token;

pub use base_uri::{BaseUri, BaseUriLayer};
pub(crate) use refresh_token::RefreshTokenLayer;
/// Layer to set up `Authorization` header depending on the config.
pub struct AuthLayer(pub(crate) Either<AddAuthorizationLayer, RefreshTokenLayer>);

impl<S> Layer<S> for AuthLayer {
    type Service =
        Either<<AddAuthorizationLayer as Layer<S>>::Service, <RefreshTokenLayer as Layer<S>>::Service>;

    fn layer(&self, inner: S) -> Self::Service {
        self.0.layer(inner)
    }
}
