use std::{
    pin::Pin,
    task::{Context, Poll},
};


use futures::{ready, Future};
use http::{header::AUTHORIZATION, Request};
use hyper::Body;
use pin_project::pin_project;
use tower::{layer::Layer, BoxError, Service};

use crate::{config::Authentication, Result};

/// `Layer` to decorate the request with `Authorization` header.
pub struct AuthLayer {
    auth: Authentication,
}

impl AuthLayer {
    pub(crate) fn new(auth: Authentication) -> Self {
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
    auth: Authentication,
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
            // If using authorization header, attach the updated value.
            auth.to_header().await.map_err(BoxError::from).map(|opt| {
                if let Some(value) = opt {
                    req.headers_mut().insert(AUTHORIZATION, value);
                }
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
