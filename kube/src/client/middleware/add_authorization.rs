// Borrowing from https://github.com/tower-rs/tower-http/pull/95
// TODO Use `tower-http`'s version once released
use std::task::{Context, Poll};

use http::{HeaderValue, Request};
use tower::{layer::Layer, Service};

#[derive(Debug, Clone)]
pub struct AddAuthorizationLayer {
    value: HeaderValue,
}

impl AddAuthorizationLayer {
    pub fn basic(username: &str, password: &str) -> Self {
        let encoded = base64::encode(format!("{}:{}", username, password));
        let mut value = HeaderValue::from_str(&format!("Basic {}", encoded)).unwrap();
        value.set_sensitive(true);
        Self { value }
    }

    pub fn bearer(token: &str) -> Self {
        let mut value =
            HeaderValue::from_str(&format!("Bearer {}", token)).expect("token is not valid header");
        value.set_sensitive(true);
        Self { value }
    }
}

impl<S> Layer<S> for AddAuthorizationLayer {
    type Service = AddAuthorization<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AddAuthorization {
            inner,
            value: self.value.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AddAuthorization<S> {
    inner: S,
    value: HeaderValue,
}

impl<S, ReqBody> Service<Request<ReqBody>> for AddAuthorization<S>
where
    S: Service<Request<ReqBody>>,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        req.headers_mut()
            .insert(http::header::AUTHORIZATION, self.value.clone());
        self.inner.call(req)
    }
}
