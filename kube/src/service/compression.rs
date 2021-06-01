// Couldn't use [Decompression layer](https://github.com/tower-rs/tower-http/pull/41) from tower-http
// because it changes the response body type and supporting that requires adding type parameter to `Client`.

use std::io::{Error as IoError, ErrorKind as IoErrorKind};

use async_compression::tokio::bufread::GzipDecoder;
use futures::TryStreamExt;
use http::{
    header::{Entry, HeaderValue, ACCEPT_ENCODING, CONTENT_ENCODING, CONTENT_LENGTH, RANGE},
    Request, Response,
};
use tokio_util::io::{ReaderStream, StreamReader};

/// Set `Accept-Encoding: gzip` if not already set.
/// Note that Kubernetes doesn't compress the response by default yet.
/// It's behind `APIResponseCompression` feature gate which is in `Beta` since `1.16`.
/// See https://kubernetes.io/docs/reference/command-line-tools-reference/feature-gates/
pub fn accept_compressed<B: http_body::Body>(mut req: Request<B>) -> Request<B> {
    if !req.headers().contains_key(RANGE) {
        if let Entry::Vacant(entry) = req.headers_mut().entry(ACCEPT_ENCODING) {
            entry.insert(HeaderValue::from_static("gzip"));
        }
    }
    req
}

/// Transparently decompresses compressed response.
pub fn maybe_decompress<B: http_body::Body>(res: Response<B>) -> Response<B> {
    let (mut parts, body) = res.into_parts();
    if let Entry::Occupied(entry) = parts.headers.entry(CONTENT_ENCODING) {
        if entry.get().as_bytes() != b"gzip" {
            return Response::from_parts(parts, body);
        }

        entry.remove();
        parts.headers.remove(CONTENT_LENGTH);
        let stream = ReaderStream::new(GzipDecoder::new(StreamReader::new(
            body.map_err(|e| IoError::new(IoErrorKind::Other, e)),
        )));
        Response::from_parts(parts, hyper::Body::wrap_stream(stream))
    } else {
        Response::from_parts(parts, body)
    }
}
