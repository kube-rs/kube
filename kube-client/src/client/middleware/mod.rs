//! Middleware types returned from `ConfigExt` methods.
use tower::{filter::AsyncFilterLayer, util::Either, Layer};
pub(crate) use tower_http::auth::AddAuthorizationLayer;

mod base_uri;
mod extra_headers;

pub use base_uri::{BaseUri, BaseUriLayer};
pub use extra_headers::{ExtraHeaders, ExtraHeadersLayer};

use super::auth::RefreshableToken;
/// Layer to set up `Authorization` header depending on the config.
pub struct AuthLayer(pub(crate) Either<AddAuthorizationLayer, AsyncFilterLayer<RefreshableToken>>);

impl<S> Layer<S> for AuthLayer {
    type Service = Either<
        <AddAuthorizationLayer as Layer<S>>::Service,
        <AsyncFilterLayer<RefreshableToken> as Layer<S>>::Service,
    >;

    fn layer(&self, inner: S) -> Self::Service {
        self.0.layer(inner)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use std::{matches, sync::Arc};

    use chrono::{Duration, Utc};
    use futures::pin_mut;
    use http::{header::AUTHORIZATION, HeaderValue, Request, Response};
    use hyper::Body;
    use secrecy::SecretString;
    use tokio::sync::Mutex;
    use tokio_test::assert_ready_ok;
    use tower::filter::AsyncFilterLayer;
    use tower_test::{mock, mock::Handle};

    use crate::{client::AuthError, config::AuthInfo};

    #[tokio::test(flavor = "current_thread")]
    async fn valid_token() {
        const TOKEN: &str = "test";
        let auth = test_token(TOKEN.into());
        let (mut service, handle): (_, Handle<Request<hyper::Body>, Response<hyper::Body>>) =
            mock::spawn_layer(AsyncFilterLayer::new(auth));

        let spawned = tokio::spawn(async move {
            // Receive the requests and respond
            pin_mut!(handle);
            let (request, send) = handle.next_request().await.expect("service not called");
            assert_eq!(
                request.headers().get(AUTHORIZATION).unwrap(),
                HeaderValue::try_from(format!("Bearer {}", TOKEN)).unwrap()
            );
            send.send_response(Response::builder().body(Body::empty()).unwrap());
        });

        assert_ready_ok!(service.poll_ready());
        service
            .call(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        spawned.await.unwrap();
    }

    #[tokio::test(flavor = "current_thread")]
    async fn invalid_token() {
        const TOKEN: &str = "\n";
        let auth = test_token(TOKEN.into());
        let (mut service, _handle) =
            mock::spawn_layer::<Request<Body>, Response<Body>, _>(AsyncFilterLayer::new(auth));
        let err = service
            .call(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap_err();

        assert!(err.is::<AuthError>());
        assert!(matches!(
            *err.downcast::<AuthError>().unwrap(),
            AuthError::InvalidBearerToken(_)
        ));
    }

    fn test_token(token: String) -> RefreshableToken {
        let expiry = Utc::now() + Duration::seconds(60 * 60);
        let secret_token = SecretString::from(token);
        let info = AuthInfo {
            token: Some(secret_token.clone()),
            ..Default::default()
        };
        RefreshableToken::Exec(Arc::new(Mutex::new((secret_token, expiry, info))))
    }
}
