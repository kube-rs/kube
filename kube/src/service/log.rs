use std::task::{Context, Poll};

use http::Request;
use tower::Service;

// `Clone` so that it can be composed with `AuthLayer`.
/// Example service to log complete request before sending.
/// Can be used to support better logging of API calls.
/// https://github.com/clux/kube-rs/issues/26
#[derive(Clone)]
pub struct LogRequest<S>
where
    S: Clone,
{
    service: S,
}

impl<S> LogRequest<S>
where
    S: Clone,
{
    /// Create `LogRequest` service wrapping `service`.
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S, B> Service<Request<B>> for LogRequest<S>
where
    S: Service<Request<B>> + Clone,
    B: http_body::Body,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        tracing::debug!("{} {}", req.method(), req.uri());
        self.service.call(req)
    }
}
