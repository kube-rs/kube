use bytes::Bytes;
use http::{Request, Response};
use hyper::{self, client::HttpConnector};
use hyper_timeout::TimeoutConnector;
pub use kube_core::response::Status;
use tower::{util::BoxService, BoxError, Layer, Service, ServiceBuilder};
use tower_http::{
    classify::ServerErrorsFailureClass, map_response_body::MapResponseBodyLayer, trace::TraceLayer,
};

use crate::{client::ConfigExt, Client, Config, Error, Result};

/// HTTP body of a dynamic backing type.
///
/// The suggested implementation type is [`hyper::Body`].
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
        Svc: Service<Request<hyper::Body>>,
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
        Svc: Service<Request<hyper::Body>, Response = Response<B>> + Send + 'static,
        Svc::Future: Send + 'static,
        Svc::Error: Into<BoxError>,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Client::new(self.service, self.default_ns)
    }
}

impl TryFrom<Config> for ClientBuilder<BoxService<Request<hyper::Body>, Response<Box<DynBody>>, BoxError>> {
    type Error = Error;

    /// Builds a default [`ClientBuilder`] stack from a given configuration
    fn try_from(config: Config) -> Result<Self> {
        use std::time::Duration;

        use http::header::HeaderMap;
        use tracing::Span;

        let default_ns = config.default_namespace.clone();

        let client: hyper::Client<_, hyper::Body> = {
            let mut connector = HttpConnector::new();
            connector.enforce_http(false);

            // Current TLS feature precedence when more than one are set:
            // 1. openssl-tls
            // 2. rustls-tls
            // Create a custom client to use something else.
            // If TLS features are not enabled, http connector will be used.
            #[cfg(feature = "openssl-tls")]
            let connector = config.openssl_https_connector_with_connector(connector)?;
            #[cfg(all(not(feature = "openssl-tls"), feature = "rustls-tls"))]
            let connector = hyper_rustls::HttpsConnector::from((
                connector,
                std::sync::Arc::new(config.rustls_client_config()?),
            ));

            let mut connector = TimeoutConnector::new(connector);

            // Set the timeout for the client and fallback to default deprecated timeout until it's removed
            #[allow(deprecated)]
            {
                connector.set_connect_timeout(config.connect_timeout.or(config.timeout));
                connector.set_read_timeout(config.read_timeout.or(config.timeout));
                connector.set_write_timeout(config.write_timeout);
            }

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
            .option_layer(config.auth_layer()?)
            .layer(config.extra_headers_layer()?)
            .layer(
                // Attribute names follow [Semantic Conventions].
                // [Semantic Conventions]: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/semantic_conventions/http.md
                TraceLayer::new_for_http()
                    .make_span_with(|req: &Request<hyper::Body>| {
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
                    .on_request(|_req: &Request<hyper::Body>, _span: &Span| {
                        tracing::debug!("requesting");
                    })
                    .on_response(|res: &Response<hyper::Body>, _latency: Duration, span: &Span| {
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

        Ok(Self::new(
            BoxService::new(
                MapResponseBodyLayer::new(|body| {
                    Box::new(http_body::Body::map_err(body, BoxError::from)) as Box<DynBody>
                })
                .layer(service),
            ),
            default_ns,
        ))
    }
}
