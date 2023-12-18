use bytes::Bytes;
use http::{header::HeaderMap, Request, Response};
use http_body_util::Full;
use hyper::{
    self,
    client::{connect::Connection, HttpConnector},
};
use hyper_timeout::TimeoutConnector;
pub use kube_core::response::Status;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tower::{util::BoxService, BoxError, Layer, Service, ServiceBuilder};
use tower_http::{
    classify::ServerErrorsFailureClass, map_response_body::MapResponseBodyLayer, trace::TraceLayer,
};
use tracing::Span;

use crate::{client::ConfigExt, Client, Config, Error, Result};

/// HTTP body of a dynamic backing type.
///
/// The suggested implementation type is [`Full<Bytes>`].
pub type DynBody = dyn http_body::Body<Data = Bytes, Error = BoxError> + Send + Unpin;

/// Builder for [`Client`] instances with customized [tower](`Service`) middleware.
pub struct ClientBuilder<Svc> {
    service: Svc,
    default_ns: String,
}

impl<Svc> ClientBuilder<Svc> {
    /// Construct a [`ClientBuilder`] from scratch with a fully custom [`Service`] stack.
    ///
    /// This method is only intended for advanced use cases, most users will want to use [`ClientBuilder::try_from`] instead,
    /// which provides a default stack as a starting point.
    pub fn new(service: Svc, default_namespace: impl Into<String>) -> Self
    where
        Svc: Service<Request<Full<Bytes>>>,
    {
        Self {
            service,
            default_ns: default_namespace.into(),
        }
    }

    /// Add a [`Layer`] to the current [`Service`] stack.
    pub fn with_layer<L: Layer<Svc>>(self, layer: &L) -> ClientBuilder<L::Service> {
        let Self {
            service: stack,
            default_ns,
        } = self;
        ClientBuilder {
            service: layer.layer(stack),
            default_ns,
        }
    }

    /// Build a [`Client`] instance with the current [`Service`] stack.
    pub fn build<B>(self) -> Client
    where
        Svc: Service<Request<Full<Bytes>>, Response = Response<B>> + Send + 'static,
        Svc::Future: Send + 'static,
        Svc::Error: Into<BoxError>,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Client::new(self.service, self.default_ns)
    }
}

pub type GenericService = BoxService<Request<Full<Bytes>>, Response<Box<DynBody>>, BoxError>;

impl TryFrom<Config> for ClientBuilder<GenericService> {
    type Error = Error;

    /// Builds a default [`ClientBuilder`] stack from a given configuration
    fn try_from(config: Config) -> Result<Self> {
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);

        #[cfg(feature = "socks5")]
        if let Some(proxy_addr) = config.proxy_url.clone() {
            let connector = hyper_socks2::SocksConnector {
                proxy_addr,
                auth: None,
                connector,
            };

            return make_generic_builder(connector, config);
        }

        make_generic_builder(connector, config)
    }
}

/// Helper function for implementation of [`TryFrom<Config>`] for [`ClientBuilder`].
/// Ignores [`Config::proxy_url`], which at this point is already handled.
fn make_generic_builder<H>(connector: H, config: Config) -> Result<ClientBuilder<GenericService>, Error>
where
    H: 'static + Clone + Send + Sync + Service<http::Uri>,
    H::Response: 'static + Connection + AsyncRead + AsyncWrite + Send + Unpin,
    H::Future: 'static + Send,
    H::Error: 'static + Send + Sync + std::error::Error,
{
    let default_ns = config.default_namespace.clone();
    let auth_layer = config.auth_layer()?;

    let client: hyper::Client<_, Full<Bytes>> = {
        // Current TLS feature precedence when more than one are set:
        // 1. rustls-tls
        // 2. openssl-tls
        // Create a custom client to use something else.
        // If TLS features are not enabled, http connector will be used.
        #[cfg(feature = "rustls-tls")]
        let connector = config.rustls_https_connector_with_connector(connector)?;
        #[cfg(all(not(feature = "rustls-tls"), feature = "openssl-tls"))]
        let connector = config.openssl_https_connector_with_connector(connector)?;
        #[cfg(all(not(feature = "rustls-tls"), not(feature = "openssl-tls")))]
        if config.cluster_url.scheme() == Some(&http::uri::Scheme::HTTPS) {
            // no tls stack situation only works with http scheme
            return Err(Error::TlsRequired);
        }

        let mut connector = TimeoutConnector::new(connector);

        // Set the timeouts for the client
        connector.set_connect_timeout(config.connect_timeout);
        connector.set_read_timeout(config.read_timeout);
        connector.set_write_timeout(config.write_timeout);

        hyper::Client::builder().build(connector)
    };

    let stack = ServiceBuilder::new().layer(config.base_uri_layer()).into_inner();
    #[cfg(feature = "gzip")]
    let stack = ServiceBuilder::new()
        .layer(stack)
        .layer(tower_http::decompression::DecompressionLayer::new())
        .into_inner();

    let service = ServiceBuilder::new()
        .layer(stack)
        .option_layer(auth_layer)
        .layer(config.extra_headers_layer()?)
        .layer(
            // Attribute names follow [Semantic Conventions].
            // [Semantic Conventions]: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/semantic_conventions/http.md
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<Full<Bytes>>| {
                    tracing::debug_span!(
                        "HTTP",
                         http.method = %req.method(),
                         http.url = %req.uri(),
                         http.status_code = tracing::field::Empty,
                         otel.name = req.extensions().get::<&'static str>().unwrap_or(&"HTTP"),
                         otel.kind = "client",
                         otel.status_code = tracing::field::Empty,
                    )
                })
                .on_request(|_req: &Request<Full<Bytes>>, _span: &Span| {
                    tracing::debug!("requesting");
                })
                .on_response(|res: &Response<Full<Bytes>>, _latency: Duration, span: &Span| {
                    let status = res.status();
                    span.record("http.status_code", status.as_u16());
                    if status.is_client_error() || status.is_server_error() {
                        span.record("otel.status_code", "ERROR");
                    }
                })
                // Explicitly disable `on_body_chunk`. The default does nothing.
                .on_body_chunk(())
                .on_eos(|_: Option<&HeaderMap>, _duration: Duration, _span: &Span| {
                    tracing::debug!("stream closed");
                })
                .on_failure(|ec: ServerErrorsFailureClass, _latency: Duration, span: &Span| {
                    // Called when
                    // - Calling the inner service errored
                    // - Polling `Body` errored
                    // - the response was classified as failure (5xx)
                    // - End of stream was classified as failure
                    span.record("otel.status_code", "ERROR");
                    match ec {
                        ServerErrorsFailureClass::StatusCode(status) => {
                            span.record("http.status_code", status.as_u16());
                            tracing::error!("failed with status {}", status)
                        }
                        ServerErrorsFailureClass::Error(err) => {
                            tracing::error!("failed with error {}", err)
                        }
                    }
                }),
        )
        .service(client);

    Ok(ClientBuilder::new(
        BoxService::new(
            MapResponseBodyLayer::new(|body| {
                Box::new(http_body::Body::map_err(body, BoxError::from)) as Box<DynBody>
            })
            .layer(service),
        ),
        default_ns,
    ))
}
