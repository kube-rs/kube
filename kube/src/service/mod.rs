//! Abstracts the connection to Kubernetes API server.

mod auth;
#[cfg(feature = "gzip")] mod compression;
mod headers;
mod log;
mod tls;
mod url;

use self::{log::LogRequest, url::set_cluster_url};
use auth::AuthLayer;
#[cfg(feature = "gzip")] use compression::{accept_compressed, maybe_decompress};
use headers::set_default_headers;
use tls::HttpsConnector;

use std::convert::{TryFrom, TryInto};

use http::{HeaderValue, Request, Response};
use hyper::{Body, Client as HyperClient};
use hyper_timeout::TimeoutConnector;
use tower::{buffer::Buffer, util::BoxService, BoxError, ServiceBuilder};

use crate::{error::ConfigError, Config, Error, Result};
use auth::Authentication;

// - `Buffer` for cheap clone
// - `BoxService` to avoid type parameters in `Client`
type InnerService = Buffer<BoxService<Request<Body>, Response<Body>, BoxError>, Request<Body>>;

#[derive(Clone)]
/// Abstracts how [`Client`](crate::Client) communicates with the Kubernetes API server.
///
/// The default service uses [`hyper::Client`] and can be created from [`Config`](crate::Config)
/// with [`Service::try_from`].
pub struct Service {
    inner: InnerService,
}

impl Service {
    /// Create a custom `Service`.
    pub fn new<S>(inner: S) -> Self
    where
        S: tower::Service<Request<Body>, Response = Response<Body>, Error = BoxError> + Send + 'static,
        S::Future: Send + 'static,
    {
        Self {
            inner: Buffer::new(BoxService::new(inner), 1024),
        }
    }
}

impl tower::Service<Request<Body>> for Service {
    type Error = <InnerService as tower::Service<Request<Body>>>::Error;
    type Future = <InnerService as tower::Service<Request<Body>>>::Future;
    type Response = <InnerService as tower::Service<Request<Body>>>::Response;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        self.inner.call(req)
    }
}

impl TryFrom<Config> for Service {
    type Error = Error;

    /// Convert [`Config`] into a [`Service`]
    fn try_from(config: Config) -> Result<Self> {
        let cluster_url = config.cluster_url.clone();
        let mut default_headers = config.headers.clone();
        let timeout = config.timeout;

        // AuthLayer is not necessary unless `RefreshableToken`
        let maybe_auth = match Authentication::try_from(&config.auth_info)? {
            Authentication::None => None,
            Authentication::Basic(s) => {
                let mut value =
                    HeaderValue::try_from(format!("Basic {}", &s)).map_err(ConfigError::InvalidBasicAuth)?;
                value.set_sensitive(true);
                default_headers.insert(http::header::AUTHORIZATION, value);
                None
            }
            Authentication::Token(s) => {
                let mut value = HeaderValue::try_from(format!("Bearer {}", &s))
                    .map_err(ConfigError::InvalidBearerToken)?;
                value.set_sensitive(true);
                default_headers.insert(http::header::AUTHORIZATION, value);
                None
            }
            Authentication::RefreshableToken(r) => Some(AuthLayer::new(r)),
        };

        let common = ServiceBuilder::new()
            .map_request(move |r| set_cluster_url(r, &cluster_url))
            .map_request(move |r| set_default_headers(r, default_headers.clone()))
            .into_inner();

        #[cfg(feature = "gzip")]
        let common = ServiceBuilder::new()
            .layer(common)
            .map_request(accept_compressed)
            .map_response(maybe_decompress)
            .into_inner();

        let https: HttpsConnector<_> = config.try_into()?;
        let mut connector = TimeoutConnector::new(https);
        if let Some(timeout) = timeout {
            // reqwest's timeout is applied from when the request stars connecting until
            // the response body has finished.
            // Setting both connect and read timeout should be close enough.
            connector.set_connect_timeout(Some(timeout));
            // Timeout for reading the response.
            connector.set_read_timeout(Some(timeout));
        }
        let client: HyperClient<_, Body> = HyperClient::builder().build(connector);

        let inner = ServiceBuilder::new()
            .layer(common)
            .option_layer(maybe_auth)
            .layer(tower::layer::layer_fn(LogRequest::new))
            .service(client);
        Ok(Self::new(inner))
    }
}
