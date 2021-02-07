use std::task::{Context, Poll};

use http::Request;
use hyper::Body;
use tower::Service;

/// Example service to log complete request before sending.
/// Can be used to support better logging of API calls.
/// https://github.com/clux/kube-rs/issues/26
pub struct LogRequest<S> {
    service: S,
}

impl<S> LogRequest<S> {
    /// Create `LogRequest` service wrapping `service`.
    pub fn new(service: S) -> Self {
        Self { service }
    }
}

impl<S> Service<Request<Body>> for LogRequest<S>
where
    S: Service<Request<Body>>,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        trace!("{} {} {:?}", req.method(), req.uri(), req.body());
        self.service.call(req)
    }
}
