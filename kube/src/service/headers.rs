use http::{header::HeaderMap, Request};
use hyper::Body;

// TODO Let users use this easily, deprecate `headers` config, and remove from default.
/// Set default headers.
pub fn set_default_headers(req: Request<Body>, mut headers: HeaderMap) -> Request<Body> {
    let (mut parts, body) = req.into_parts();
    headers.extend(parts.headers.into_iter());
    parts.headers = headers;
    Request::from_parts(parts, body)
}
