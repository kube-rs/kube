use http::{header::HeaderMap, Request};
use tower::{Layer, Service};

// TODO Remove this and `headers` field from `Config`.
/// Layer that applies [`SetHeaders`] which sets the provided headers to each request.
#[derive(Debug, Clone)]
pub struct SetHeadersLayer {
    headers: HeaderMap,
}

impl SetHeadersLayer {
    /// Create a new [`SetHeadersLayer`].
    pub fn new(headers: HeaderMap) -> Self {
        Self { headers }
    }
}

impl<S> Layer<S> for SetHeadersLayer {
    type Service = SetHeaders<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SetHeaders {
            headers: self.headers.clone(),
            inner,
        }
    }
}

/// Middleware that set headers.
#[derive(Debug, Clone)]
pub struct SetHeaders<S> {
    headers: HeaderMap,
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for SetHeaders<S>
where
    S: Service<Request<ReqBody>>,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let (mut parts, body) = req.into_parts();
        let mut headers = self.headers.clone();
        headers.extend(parts.headers.into_iter());
        parts.headers = headers;
        self.inner.call(Request::from_parts(parts, body))
    }
}
