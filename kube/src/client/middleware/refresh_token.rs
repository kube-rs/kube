use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures::{ready, Future};
use http::{header::AUTHORIZATION, Request, Response};
use pin_project::pin_project;
use tower::{layer::Layer, BoxError, Service};

use crate::{client::auth::RefreshableToken, Result};

/// `Layer` to decorate the request with `Authorization` header with refreshable token.
/// Token is refreshed automatically when necessary.
pub struct RefreshTokenLayer {
    refreshable: RefreshableToken,
}

impl RefreshTokenLayer {
    pub(crate) fn new(refreshable: RefreshableToken) -> Self {
        Self { refreshable }
    }
}

impl<S> Layer<S> for RefreshTokenLayer {
    type Service = RefreshToken<S>;

    fn layer(&self, service: S) -> Self::Service {
        RefreshToken {
            refreshable: self.refreshable.clone(),
            service,
        }
    }
}

pub struct RefreshToken<S> {
    refreshable: RefreshableToken,
    service: S,
}

impl<S, ReqB, ResB> Service<Request<ReqB>> for RefreshToken<S>
where
    S: Service<Request<ReqB>, Response = Response<ResB>> + Clone,
    S::Error: Into<BoxError>,
    ReqB: http_body::Body + Send + Unpin + 'static,
    ResB: http_body::Body,
{
    type Error = BoxError;
    type Future = RefreshTokenFuture<S, ReqB>;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, mut req: Request<ReqB>) -> Self::Future {
        // Comment from `AsyncFilter`
        // > In case the inner service has state that's driven to readiness and
        // > not tracked by clones (such as `Buffer`), pass the version we have
        // > already called `poll_ready` on into the future, and leave its clone
        // > behind.
        let service = self.service.clone();
        let service = std::mem::replace(&mut self.service, service);

        let auth = self.refreshable.clone();
        let request = async move {
            auth.to_header().await.map_err(BoxError::from).map(|value| {
                req.headers_mut().insert(AUTHORIZATION, value);
                req
            })
        };

        RefreshTokenFuture {
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

type RequestFuture<B> = Pin<Box<dyn Future<Output = Result<Request<B>, BoxError>> + Send>>;

#[pin_project]
pub struct RefreshTokenFuture<S, B>
where
    S: Service<Request<B>>,
    B: http_body::Body,
{
    #[pin]
    state: State<RequestFuture<B>, S::Future>,
    service: S,
}

impl<S, B> Future for RefreshTokenFuture<S, B>
where
    S: Service<Request<B>>,
    S::Error: Into<BoxError>,
    B: http_body::Body,
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

    use std::{convert::TryFrom, matches, sync::Arc};

    use chrono::{Duration, Utc};
    use futures::pin_mut;
    use http::{HeaderValue, Request, Response};
    use hyper::Body;
    use tokio::sync::Mutex;
    use tokio_test::assert_ready_ok;
    use tower_test::{mock, mock::Handle};

    use crate::{config::AuthInfo, error::ConfigError, Error};

    #[tokio::test(flavor = "current_thread")]
    async fn valid_token() {
        const TOKEN: &str = "test";
        let auth = test_token(TOKEN.into());
        let (mut service, handle): (_, Handle<Request<hyper::Body>, Response<hyper::Body>>) =
            mock::spawn_layer(RefreshTokenLayer::new(auth));

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
            mock::spawn_layer::<Request<Body>, Response<Body>, _>(RefreshTokenLayer::new(auth));
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
