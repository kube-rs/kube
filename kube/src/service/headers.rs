use http::{header::HeaderMap, Request};

// TODO Let users use this easily, deprecate `headers` config, and remove from default.
/// Set default headers.
pub fn set_default_headers<B: http_body::Body>(req: Request<B>, mut headers: HeaderMap) -> Request<B> {
    let (mut parts, body) = req.into_parts();
    headers.extend(parts.headers.into_iter());
    parts.headers = headers;
    Request::from_parts(parts, body)
}
