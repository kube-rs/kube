use bytes::Bytes;
use http::{Request, Response, header::HeaderMap};
use hyper::{
    body::Incoming,
    rt::{Read, Write},
};
use hyper_timeout::TimeoutConnector;

use hyper_util::{
    client::legacy::connect::{Connection, HttpConnector},
    rt::{TokioExecutor, TokioTimer},
};

use jiff::Timestamp;
use std::time::Duration;
use tower::{BoxError, Layer, Service, ServiceBuilder, ServiceExt as _, retry::RetryLayer, util::BoxService};
use tower_http::{ServiceExt as _, classify::ServerErrorsFailureClass, trace::TraceLayer};
use tracing::Span;

use super::body::Body;
use crate::{Client, Config, Error, Result, client::{ConfigExt, retry::RetryPolicy}};

/// HTTP body of a dynamic backing type.
///
/// The suggested implementation type is [`crate::client::Body`].
pub type DynBody = dyn http_body::Body<Data = Bytes, Error = BoxError> + Send + Unpin;

/// Builder for [`Client`] instances with customized [tower](`Service`) middleware.
pub struct ClientBuilder<Svc> {
    service: Svc,
    upgrade_service: Option<GenericService>,
    default_ns: String,
    valid_until: Option<Timestamp>,
}

impl<Svc> ClientBuilder<Svc> {
    /// Construct a [`ClientBuilder`] from scratch with a fully custom [`Service`] stack.
    ///
    /// This method is only intended for advanced use cases, most users will want to use [`ClientBuilder::try_from`] instead,
    /// which provides a default stack as a starting point.
    pub fn new(service: Svc, default_namespace: impl Into<String>) -> Self
    where
        Svc: Service<Request<Body>>,
    {
        Self {
            service,
            upgrade_service: None,
            default_ns: default_namespace.into(),
            valid_until: None,
        }
    }

    /// Add a [`Layer`] to the current [`Service`] stack.
    ///
    /// The layer is applied to the primary [`Service`] only. If an upgrade
    /// service has been set via [`with_upgrade_service`](Self::with_upgrade_service)
    /// it is left untouched; users wanting to layer both must apply the
    /// layer to each service themselves before calling
    /// [`with_upgrade_service`].
    pub fn with_layer<L: Layer<Svc>>(self, layer: &L) -> ClientBuilder<L::Service> {
        let Self {
            service: stack,
            upgrade_service,
            default_ns,
            valid_until,
        } = self;
        ClientBuilder {
            service: layer.layer(stack),
            upgrade_service,
            default_ns,
            valid_until,
        }
    }

    /// Sets an expiration timestamp for the client.
    pub fn with_valid_until(self, valid_until: Option<Timestamp>) -> Self {
        ClientBuilder {
            service: self.service,
            upgrade_service: self.upgrade_service,
            default_ns: self.default_ns,
            valid_until,
        }
    }

    /// Provide a separate [`Service`] used by the upgrade transport that
    /// backs exec, attach, and port-forward.
    ///
    /// The supplied service is the same shape as [`GenericService`], the
    /// boxed service produced by the default builder stack. Custom-service
    /// users that do not naturally arrive at this shape can instead call
    /// [`Client::new_with_upgrade`] directly.
    ///
    /// This is required only if the primary service may negotiate HTTP/2
    /// *and* the application also uses upgrade subresources. HTTP/1.1
    /// upgrades are unrepresentable on an HTTP/2 connection, so the
    /// upgrade transport must offer only HTTP/1.1.
    pub fn with_upgrade_service(self, upgrade_service: GenericService) -> Self {
        ClientBuilder {
            service: self.service,
            upgrade_service: Some(upgrade_service),
            default_ns: self.default_ns,
            valid_until: self.valid_until,
        }
    }

    /// Build a [`Client`] instance with the current [`Service`] stack.
    pub fn build<B>(self) -> Client
    where
        Svc: Service<Request<Body>, Response = Response<B>> + Send + 'static,
        Svc::Future: Send + 'static,
        Svc::Error: Into<BoxError>,
        B: http_body::Body<Data = bytes::Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        let Self {
            service,
            upgrade_service,
            default_ns,
            valid_until,
        } = self;
        match upgrade_service {
            Some(upgrade) => Client::new_with_upgrade(service, upgrade, default_ns),
            None => Client::new(service, default_ns),
        }
        .with_valid_until(valid_until)
    }
}

pub type GenericService = BoxService<Request<Body>, Response<Box<DynBody>>, BoxError>;

impl TryFrom<Config> for ClientBuilder<GenericService> {
    type Error = Error;

    /// Builds a default [`ClientBuilder`] stack from a given configuration
    fn try_from(config: Config) -> Result<Self> {
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);

        #[cfg(all(feature = "aws-lc-rs", feature = "rustls-tls"))]
        {
            if rustls::crypto::CryptoProvider::get_default().is_none() {
                // the only error here is if it's been initialized in between: we can ignore it
                // since our semantic is only to set the default value if it does not exist.
                let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
            }
        }

        match config.proxy_url.as_ref() {
            Some(proxy_url) if proxy_url.scheme_str() == Some("socks5") => {
                #[cfg(feature = "socks5")]
                {
                    let connector = hyper_util::client::legacy::connect::proxy::SocksV5::new(
                        proxy_url.clone(),
                        connector,
                    );
                    make_generic_builder(connector, config)
                }

                #[cfg(not(feature = "socks5"))]
                Err(Error::ProxyProtocolDisabled {
                    proxy_url: proxy_url.clone(),
                    protocol_feature: "kube/socks5",
                })
            }

            Some(proxy_url) if proxy_url.scheme_str() == Some("http") => {
                #[cfg(feature = "http-proxy")]
                {
                    let mut connector =
                        hyper_util::client::legacy::connect::proxy::Tunnel::new(proxy_url.clone(), connector);

                    if let Some(authority) = proxy_url.authority() {
                        if let Some((userinfo, _)) = authority.as_str().split_once('@') {
                            use base64::Engine;
                            use http::HeaderValue;

                            let value = format!(
                                "Basic {}",
                                base64::engine::general_purpose::STANDARD.encode(userinfo)
                            );
                            let header = HeaderValue::from_str(&value).unwrap();
                            connector = connector.with_auth(header);
                        }
                    }

                    make_generic_builder(connector, config)
                }

                #[cfg(not(feature = "http-proxy"))]
                Err(Error::ProxyProtocolDisabled {
                    proxy_url: proxy_url.clone(),
                    protocol_feature: "kube/http-proxy",
                })
            }

            Some(proxy_url) => Err(Error::ProxyProtocolUnsupported {
                proxy_url: proxy_url.clone(),
            }),

            None => make_generic_builder(connector, config),
        }
    }
}

/// Helper function for implementation of [`TryFrom<Config>`] for [`ClientBuilder`].
/// Ignores [`Config::proxy_url`], which at this point is already handled.
fn make_generic_builder<H>(connector: H, config: Config) -> Result<ClientBuilder<GenericService>, Error>
where
    H: 'static + Clone + Send + Sync + Service<http::Uri>,
    H::Response: 'static + Connection + Read + Write + Send + Unpin,
    H::Future: 'static + Send,
    H::Error: 'static + Send + Sync + std::error::Error,
{
    let default_ns = config.default_namespace.clone();

    // Build two hyper clients with separate connection pools and ALPN policies:
    // - the primary, h2-capable transport for normal REST/watch/log traffic
    //   (subject to `Config::disable_http2`)
    // - an HTTP/1.1-only transport for the upgrade path used by exec, attach,
    //   and port-forward, regardless of `disable_http2`.
    //
    // Current TLS feature precedence when more than one is set:
    //   1. rustls-tls
    //   2. openssl-tls
    // If neither TLS feature is enabled, the http connector is used; only the
    // http scheme is supported in that case.
    // Compute auth and extra-headers layers once and share across both
    // transports. Calling `auth_layer()` twice would mint independent
    // `RefreshableToken` state per transport, so each path would refresh
    // tokens on its own and they'd diverge under exec-plugin or token-file
    // auth.
    let auth_layer = config.auth_layer()?;
    let extra_headers_layer = config.extra_headers_layer()?;

    // The two transports use connectors with different concrete types after
    // TLS wrapping (h2-capable vs explicit-h1 ALPN), so each path is built
    // and wrapped independently and erased to a `GenericService` here rather
    // than threading the connector type through.
    let main_service = build_main_service(
        connector.clone(),
        &config,
        auth_layer.clone(),
        extra_headers_layer.clone(),
    )?;
    let upgrade_service = build_upgrade_service(connector, &config, auth_layer, extra_headers_layer)?;

    let (_, expiration) = config.exec_identity_pem();

    let client = ClientBuilder::new(main_service, default_ns)
        .with_upgrade_service(upgrade_service)
        .with_valid_until(expiration);

    Ok(client)
}

/// Build the primary, h2-capable transport service.
///
/// Uses the dual-protocol TLS connector (rustls advertises `h2,http/1.1`
/// in ALPN; openssl currently advertises nothing -- parity work is a
/// follow-up). The hyper client carries `TokioTimer` and HTTP/2
/// keep-alive PINGs so watch streams survive idle-killing intermediaries
/// such as HAProxy.
///
/// If `Config::disable_http2` is set, falls back to building a structurally
/// identical service via [`build_upgrade_service`] -- both clients then
/// carry HTTP/1.1-only connectors but the two-client shape stays the same.
fn build_main_service<H>(
    connector: H,
    config: &Config,
    auth_layer: Option<crate::client::middleware::AuthLayer>,
    extra_headers_layer: crate::client::middleware::ExtraHeadersLayer,
) -> Result<GenericService, Error>
where
    H: 'static + Clone + Send + Sync + Service<http::Uri>,
    H::Response: 'static + Connection + Read + Write + Send + Unpin,
    H::Future: 'static + Send,
    H::Error: 'static + Send + Sync + std::error::Error,
{
    if config.disable_http2 {
        return build_upgrade_service(connector, config, auth_layer, extra_headers_layer);
    }

    #[cfg(feature = "rustls-tls")]
    let connector = config.rustls_https_connector_with_connector(connector)?;
    #[cfg(all(not(feature = "rustls-tls"), feature = "openssl-tls"))]
    let connector = config.openssl_https_connector_with_connector(connector)?;
    #[cfg(all(not(feature = "rustls-tls"), not(feature = "openssl-tls")))]
    {
        if config.cluster_url.scheme() == Some(&http::uri::Scheme::HTTPS) {
            return Err(Error::TlsRequired);
        }
    }

    let mut connector = TimeoutConnector::new(connector);
    connector.set_connect_timeout(config.connect_timeout);
    connector.set_read_timeout(config.read_timeout);
    connector.set_write_timeout(config.write_timeout);

    let mut builder = hyper_util::client::legacy::Builder::new(TokioExecutor::new());
    builder
        .timer(TokioTimer::new())
        .http2_keep_alive_interval(Duration::from_secs(30))
        .http2_keep_alive_while_idle(true);
    let client = builder.build(connector);
    wrap_with_layers(client, config, auth_layer, extra_headers_layer)
}

/// Build the HTTP/1.1-only upgrade transport service.
///
/// Used by exec, attach, and port-forward; HTTP/1.1 upgrades are
/// unrepresentable on an HTTP/2 connection. The connector explicitly
/// advertises `http/1.1` in ALPN (rustls) so the server cannot pick
/// HTTP/2 at the TLS handshake.
fn build_upgrade_service<H>(
    connector: H,
    config: &Config,
    auth_layer: Option<crate::client::middleware::AuthLayer>,
    extra_headers_layer: crate::client::middleware::ExtraHeadersLayer,
) -> Result<GenericService, Error>
where
    H: 'static + Clone + Send + Sync + Service<http::Uri>,
    H::Response: 'static + Connection + Read + Write + Send + Unpin,
    H::Future: 'static + Send,
    H::Error: 'static + Send + Sync + std::error::Error,
{
    #[cfg(feature = "rustls-tls")]
    let connector = config.rustls_https_connector_http1_only_with_connector(connector)?;
    #[cfg(all(not(feature = "rustls-tls"), feature = "openssl-tls"))]
    let connector = config.openssl_https_connector_with_connector(connector)?;
    #[cfg(all(not(feature = "rustls-tls"), not(feature = "openssl-tls")))]
    {
        if config.cluster_url.scheme() == Some(&http::uri::Scheme::HTTPS) {
            return Err(Error::TlsRequired);
        }
    }

    let mut connector = TimeoutConnector::new(connector);
    connector.set_connect_timeout(config.connect_timeout);
    connector.set_read_timeout(config.read_timeout);
    connector.set_write_timeout(config.write_timeout);

    let builder = hyper_util::client::legacy::Builder::new(TokioExecutor::new());
    let client = builder.build(connector);
    wrap_with_layers(client, config, auth_layer, extra_headers_layer)
}

/// Wrap a hyper client with the standard tower layer stack (base URI, gzip,
/// auth, extra headers, tracing) and erase to a [`GenericService`].
fn wrap_with_layers<C>(
    client: hyper_util::client::legacy::Client<C, Body>,
    config: &Config,
    auth_layer: Option<crate::client::middleware::AuthLayer>,
    extra_headers_layer: crate::client::middleware::ExtraHeadersLayer,
) -> Result<GenericService, Error>
where
    C: 'static + Clone + Send + Sync + Service<http::Uri>,
    C::Response: 'static + Connection + Read + Write + Send + Unpin,
    C::Future: 'static + Send + Unpin,
    C::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    let stack = ServiceBuilder::new().layer(config.base_uri_layer()).into_inner();
    #[cfg(feature = "gzip")]
    let stack = ServiceBuilder::new()
        .layer(stack)
        .layer(
            tower_http::decompression::DecompressionLayer::new()
                .no_br()
                .no_deflate()
                .no_zstd()
                .gzip(!config.disable_compression),
        )
        .into_inner();

    let service = ServiceBuilder::new()
        .layer(stack)
        .option_layer(config.default_retry.then_some(RetryLayer::new(RetryPolicy::server_retry())))
        .option_layer(auth_layer)
        .layer(extra_headers_layer)
        .layer(
            // Attribute names follow [Semantic Conventions].
            // [Semantic Conventions]: https://github.com/open-telemetry/opentelemetry-specification/blob/main/specification/trace/semantic_conventions/http.md
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<Body>| {
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
                .on_request(|_req: &Request<Body>, _span: &Span| {
                    tracing::debug!("requesting");
                })
                .on_response(|res: &Response<Incoming>, _latency: Duration, span: &Span| {
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
        .map_err(BoxError::from)
        .service(client);

    Ok(service
        .map_response_body(|body| {
            Box::new(http_body_util::BodyExt::map_err(body, BoxError::from)) as Box<DynBody>
        })
        .boxed())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "gzip")] use super::*;

    #[cfg(feature = "gzip")]
    #[tokio::test]
    async fn test_no_accept_encoding_header_sent_when_compression_disabled()
    -> Result<(), Box<dyn std::error::Error>> {
        use http::Uri;
        use std::net::SocketAddr;
        use tokio::net::{TcpListener, TcpStream};

        // setup a server that echoes back any encoding header value
        let addr: SocketAddr = ([127, 0, 0, 1], 0).into();
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;
        let uri: Uri = format!("http://{}", local_addr).parse()?;

        tokio::spawn(async move {
            use http_body_util::Full;
            use hyper::{server::conn::http1, service::service_fn};
            use hyper_util::rt::{TokioIo, TokioTimer};
            use std::convert::Infallible;

            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                let io: TokioIo<TcpStream> = TokioIo::new(tcp);

                tokio::spawn(async move {
                    http1::Builder::new()
                        .timer(TokioTimer::new())
                        .serve_connection(
                            io,
                            service_fn(|req| async move {
                                let response = req
                                    .headers()
                                    .get(http::header::ACCEPT_ENCODING)
                                    .map(|b| Bytes::copy_from_slice(b.as_bytes()))
                                    .unwrap_or_default();
                                Ok::<_, Infallible>(Response::new(Full::new(response)))
                            }),
                        )
                        .await
                        .unwrap();
                });
            }
        });

        // confirm gzip echoed back with default config
        let config = Config { ..Config::new(uri) };
        let client = make_generic_builder(HttpConnector::new(), config.clone())?.build();
        let response = client.request_text(http::Request::default()).await?;
        assert_eq!(&response, "gzip");

        // now disable and check empty string echoed back
        let config = Config {
            disable_compression: true,
            ..config
        };
        let client = make_generic_builder(HttpConnector::new(), config)?.build();
        let response = client.request_text(http::Request::default()).await?;
        assert_eq!(&response, "");

        Ok(())
    }
}
