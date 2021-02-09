use std::{
    pin::Pin,
    task::{Context, Poll},
};


use futures::{ready, Future};
use http::{header::AUTHORIZATION, Request};
use hyper::Body;
use pin_project::pin_project;
use tower::{layer::Layer, BoxError, Service};

use super::RefreshableToken;
use crate::Result;

/// `Layer` to decorate the request with `Authorization` header.
pub struct AuthLayer {
    auth: RefreshableToken,
}

impl AuthLayer {
    pub(crate) fn new(auth: RefreshableToken) -> Self {
        Self { auth }
    }
}

impl<S> Layer<S> for AuthLayer
where
    S: Service<Request<Body>>,
{
    type Service = AuthService<S>;

    fn layer(&self, service: S) -> Self::Service {
        AuthService {
            auth: self.auth.clone(),
            service,
        }
    }
}

pub struct AuthService<S>
where
    S: Service<Request<Body>>,
{
    auth: RefreshableToken,
    service: S,
}

impl<S> Service<Request<Body>> for AuthService<S>
where
    S: Service<Request<Body>> + Clone,
    S::Error: Into<BoxError>,
{
    type Error = BoxError;
    type Future = AuthFuture<S>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        // Comment from `AsyncFilter`
        // > In case the inner service has state that's driven to readiness and
        // > not tracked by clones (such as `Buffer`), pass the version we have
        // > already called `poll_ready` on into the future, and leave its clone
        // > behind.
        let service = self.service.clone();
        let service = std::mem::replace(&mut self.service, service);

        let auth = self.auth.clone();
        let request = async move {
            auth.to_header().await.map_err(BoxError::from).map(|value| {
                req.headers_mut().insert(AUTHORIZATION, value);
                req
            })
        };

        AuthFuture {
            state: State::Request(Box::pin(request)),
            service,
        }
    }
}

#[pin_project(project = StateProj)]
#[derive(Debug)]
enum State<F, G> {
    /// Waiting for the request future
    Request(#[pin] F),
    /// Waiting for the response future
    Response(#[pin] G),
}

type RequestFuture = Pin<Box<dyn Future<Output = Result<Request<Body>, BoxError>> + Send>>;

#[pin_project]
pub struct AuthFuture<S>
where
    S: Service<Request<Body>>,
{
    #[pin]
    state: State<RequestFuture, S::Future>,
    service: S,
}

impl<S> Future for AuthFuture<S>
where
    S: Service<Request<Body>>,
    S::Error: Into<BoxError>,
{
    type Output = Result<S::Response, BoxError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        loop {
            match this.state.as_mut().project() {
                StateProj::Request(mut req) => {
                    let request = ready!(req.as_mut().poll(cx))?;
                    let response = this.service.call(request);
                    this.state.set(State::Response(response));
                }

                StateProj::Response(response) => {
                    return response.poll(cx).map_err(Into::into);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{matches, sync::Arc};

    use chrono::{Duration, Utc};
    use futures::pin_mut;
    use http::{HeaderValue, Request, Response};
    use hyper::Body;
    use tokio::sync::Mutex;
    use tokio_test::assert_ready_ok;
    use tower_test::mock;

    use crate::{config::AuthInfo, error::ConfigError, Error};

    #[tokio::test(flavor = "current_thread")]
    async fn valid_token() {
        const TOKEN: &str = "test";
        let auth = test_token(TOKEN.into());
        let (mut service, handle) = mock::spawn_layer(AuthLayer::new(auth));

        let spawned = tokio::spawn(async move {
            // Receive the requests and respond
            pin_mut!(handle);
            let (request, send) = handle.next_request().await.expect("service not called");
            assert_eq!(
                request.headers().get(AUTHORIZATION).unwrap(),
                HeaderValue::from_str(&format!("Bearer {}", TOKEN)).unwrap()
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
            mock::spawn_layer::<Request<Body>, Response<Body>, _>(AuthLayer::new(auth));
        let err = service
            .call(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap_err();

        assert!(err.is::<Error>());
        assert!(matches!(
            *err.downcast::<Error>().unwrap(),
            Error::Kubeconfig(ConfigError::InvalidBearerToken(_))
        ));
    }

    fn test_token(token: String) -> RefreshableToken {
        let expiry = Utc::now() + Duration::seconds(60 * 60);
        let info = AuthInfo {
            token: Some(token.clone()),
            ..Default::default()
        };
        RefreshableToken::Exec(Arc::new(Mutex::new((token, expiry, info))))
    }
}
