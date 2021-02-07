//! `Service` abstracts the connection to Kubernetes API server.

mod auth;
mod headers;
mod log;
mod tls;
mod url;

use self::{log::LogRequest, url::set_cluster_url};
use headers::set_default_headers;
use tls::HttpsConnector;

use std::convert::{TryFrom, TryInto};

use http::{Request, Response};
use hyper::{client::HttpConnector, Body, Client as HyperClient};
use tower::{buffer::Buffer, util::BoxService, ServiceBuilder};

use crate::{Config, Error, Result};

// - `Buffer` for cheap clone
// - `BoxService` to avoid type parameters in `Client`
type InnerService = Buffer<BoxService<Request<Body>, Response<Body>, hyper::Error>, Request<Body>>;

#[derive(Clone)]
/// `Service` abstracts how `Client` communicates with the Kubernetes API server.
pub struct Service {
    inner: InnerService,
}

impl Service {
    /// Create a custom `Service`.
    pub fn new<S>(inner: S) -> Self
    where
        S: tower::Service<Request<Body>, Response = Response<Body>, Error = hyper::Error> + Send + 'static,
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
        let default_headers = config.headers.clone();

        let connector = config.try_into()?;
        let client: HyperClient<HttpsConnector<HttpConnector>, Body> =
            HyperClient::builder().build(connector);

        let inner = ServiceBuilder::new()
            .map_request(move |r| set_cluster_url(r, &cluster_url))
            .map_request(move |r| set_default_headers(r, default_headers.clone()))
            .layer(tower::layer::layer_fn(LogRequest::new))
            .service(client);
        Ok(Self::new(inner))
    }
}
