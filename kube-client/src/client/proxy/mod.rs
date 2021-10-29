// Most of the code was extracted from `reqwest`.
use std::{
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::Future;
use http::{uri::Scheme, HeaderValue, Uri};
use hyper::client::{
    connect::{Connected, Connection},
    HttpConnector,
};
use pin_project::pin_project;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tower::Service;

mod scheme;
#[cfg(feature = "socks-proxy")] mod socks;

trait AsyncConn: AsyncRead + AsyncWrite + Connection + Send + Sync + Unpin + 'static {}
impl<T: AsyncRead + AsyncWrite + Connection + Send + Sync + Unpin + 'static> AsyncConn for T {}

type BoxError = Box<dyn std::error::Error + Send + Sync>;
type BoxConn = Box<dyn AsyncConn>;
type Connecting = Pin<Box<dyn Future<Output = Result<Conn, BoxError>> + Send>>;

// https://golang.org/src/vendor/golang.org/x/net/http/httpproxy/proxy.go
// TODO Config should have PEM so that new conector with proxy's PEM can be created.
// TODO Extract basic auth from the proxy uri
// TODO Support `NO_PROXY` environment variable like kubectl
// TODO Ignore `HTTP_PROXY` environment when in CGI (when `REQUEST_METHOD` evar is present) for security

// TODO If proxy is missing scheme, assume http (like Go)
/// Connector that intercepts all requests.
#[derive(Clone)]
pub struct ProxyConnector {
    inner: Inner,
    proxy_uri: http::Uri,
    #[cfg(any(feature = "native-tls", feature = "rustls-tls"))]
    nodelay: bool,
}

impl ProxyConnector {
    /// Create proxy connector
    #[cfg(feature = "native-tls")]
    pub fn native_tls(
        proxy_uri: http::Uri,
        mut http: HttpConnector,
        tls: hyper_tls::native_tls::TlsConnector,
    ) -> Self {
        http.enforce_http(false);
        Self {
            inner: Inner::HttpsNativeTls { http, tls },
            proxy_uri,
            nodelay: true,
        }
    }

    /// Create proxy connector
    #[cfg(feature = "rustls-tls")]
    pub fn rustls(proxy_uri: http::Uri, mut http: HttpConnector, mut config: rustls::ClientConfig) -> Self {
        use std::sync::Arc;
        http.enforce_http(false);
        config.alpn_protocols.clear();
        let tls = Arc::new(config);
        Self {
            inner: Inner::HttpsRustls {
                http,
                tls: tls.clone(),
                proxy_tls: tls,
            },
            proxy_uri,
            nodelay: true,
        }
    }

    /// Create proxy connector
    #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
    pub fn http(proxy_uri: http::Uri, http: HttpConnector) -> Self {
        Self {
            inner: Inner::Http(http),
            proxy_uri,
        }
    }
}

#[derive(Clone)]
pub(crate) enum Inner {
    #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
    Http(HttpConnector),

    #[cfg(feature = "native-tls")]
    HttpsNativeTls {
        http: HttpConnector,
        tls: hyper_tls::native_tls::TlsConnector,
    },

    #[cfg(feature = "rustls-tls")]
    HttpsRustls {
        http: HttpConnector,
        tls: std::sync::Arc<rustls::ClientConfig>,
        proxy_tls: std::sync::Arc<rustls::ClientConfig>,
    },
}

impl ProxyConnector {
    async fn connect_via_proxy(self, dst: Uri) -> Result<Conn, BoxError> {
        let proxy_dst = self.proxy_uri.clone();
        // TODO Support optional socks proxy

        match &self.inner {
            #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
            Inner::Http(_http) => {}

            #[cfg(feature = "native-tls")]
            Inner::HttpsNativeTls { http, tls } => {
                if dst.scheme() == Some(&Scheme::HTTPS) {
                    use self::native_tls_conn::NativeTlsConn;

                    let host = dst.host().to_owned();
                    let port = dst.port_u16().unwrap_or(443);
                    let tls_connector = tokio_native_tls::TlsConnector::from(tls.clone());
                    let mut https = hyper_tls::HttpsConnector::from((http.clone(), tls_connector));
                    let conn = https.call(proxy_dst).await?;

                    tracing::trace!("tunneling HTTPS over proxy");
                    let host = host.ok_or("no host in url")?;
                    let tunneled = tunnel(conn, host, port, None).await?;
                    let tls_connector = tokio_native_tls::TlsConnector::from(tls.clone());
                    let io = tls_connector.connect(host, tunneled).await?;
                    return Ok(Conn {
                        inner: Box::new(NativeTlsConn { inner: io }),
                        is_proxy: false,
                    });
                }
            }

            #[cfg(feature = "rustls-tls")]
            Inner::HttpsRustls { http, tls, proxy_tls } => {
                if dst.scheme() == Some(&Scheme::HTTPS) {
                    use self::rustls_tls_conn::RustlsTlsConn;
                    use tokio_rustls::TlsConnector as RustlsConnector;
                    use webpki::DnsNameRef;

                    let host = dst.host().ok_or("no host in url")?.to_string();
                    let port = dst.port_u16().unwrap_or(443);
                    let mut https = hyper_rustls::HttpsConnector::from((http.clone(), proxy_tls.clone()));
                    let conn = https.call(proxy_dst).await?;

                    tracing::trace!("tunneling HTTPS over proxy");
                    let maybe_dnsname = DnsNameRef::try_from_ascii_str(&host)
                        .map(|dnsname| dnsname.to_owned())
                        .map_err(|_| "Invalid DNS Name");
                    let tunneled = tunnel(conn, &host, port, None).await?;
                    let dnsname = maybe_dnsname?;
                    let domain = rustls::ServerName::try_from(AsRef::<str>::as_ref(&dnsname))?;
                    let io = RustlsConnector::from(tls.clone())
                        .connect(domain, tunneled)
                        .await?;

                    return Ok(Conn {
                        inner: Box::new(RustlsTlsConn { inner: io }),
                        is_proxy: false,
                    });
                }
            }
        }

        self.connect_with_maybe_proxy(proxy_dst, true).await
    }

    async fn connect_with_maybe_proxy(self, dst: Uri, is_proxy: bool) -> Result<Conn, BoxError> {
        match self.inner {
            #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
            Inner::Http(mut http) => Ok(Conn {
                inner: Box::new(http.call(dst).await?),
                is_proxy,
            }),

            #[cfg(feature = "native-tls")]
            Inner::HttpsNativeTls { http, tls } => {
                let mut http = http.clone();
                // Disable Nagle's algorithm for TLS handshake
                //
                // https://www.openssl.org/docs/man1.1.1/man3/SSL_connect.html#NOTES
                if !self.nodelay && (dst.scheme() == Some(&Scheme::HTTPS)) {
                    http.set_nodelay(true);
                }

                let tls_connector = tokio_native_tls::TlsConnector::from(tls.clone());
                let mut https = hyper_tls::HttpsConnector::from((http, tls_connector));
                let io = https.call(dst).await?;

                if let hyper_tls::MaybeHttpsStream::Https(stream) = &io {
                    if !self.nodelay {
                        stream.get_ref().get_ref().get_ref().set_nodelay(false)?;
                    }
                }

                Ok(Conn {
                    inner: Box::new(io),
                    is_proxy,
                })
            }

            #[cfg(feature = "rustls-tls")]
            Inner::HttpsRustls { http, tls, proxy_tls } => {
                use self::rustls_tls_conn::RustlsTlsConn;

                let mut http = http.clone();
                // Disable Nagle's algorithm for TLS handshake
                //
                // https://www.openssl.org/docs/man1.1.1/man3/SSL_connect.html#NOTES
                if !self.nodelay && (dst.scheme() == Some(&Scheme::HTTPS)) {
                    http.set_nodelay(true);
                }

                let mut http = hyper_rustls::HttpsConnector::from((http, tls.clone()));
                let io = http.call(dst).await?;

                if let hyper_rustls::MaybeHttpsStream::Https(stream) = io {
                    if !self.nodelay {
                        let (io, _) = stream.get_ref();
                        io.set_nodelay(false)?;
                    }
                    Ok(Conn {
                        inner: Box::new(RustlsTlsConn { inner: stream }),
                        is_proxy,
                    })
                } else {
                    Ok(Conn {
                        inner: Box::new(io),
                        is_proxy,
                    })
                }
            }
        }
    }
}

impl Service<Uri> for ProxyConnector {
    type Error = BoxError;
    type Future = Connecting;
    type Response = Conn;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        tracing::debug!("starting new connection: {:?}", dst);
        Box::pin(self.clone().connect_via_proxy(dst))
    }
}


async fn tunnel<T>(mut conn: T, host: &str, port: u16, auth: Option<HeaderValue>) -> Result<T, BoxError>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // https://developer.mozilla.org/en-US/docs/Web/HTTP/Methods/CONNECT
    let mut buf = format!(
        "\
         CONNECT {0}:{1} HTTP/1.1\r\n\
         Host: {0}:{1}\r\n\
         ",
        host, port
    )
    .into_bytes();

    // proxy-authorization
    if let Some(value) = auth {
        tracing::debug!("tunnel to {}:{} using basic auth", host, port);
        buf.extend_from_slice(b"Proxy-Authorization: ");
        buf.extend_from_slice(value.as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    // headers end
    buf.extend_from_slice(b"\r\n");

    conn.write_all(&buf).await?;

    let mut buf = [0; 8192];
    let mut pos = 0;

    loop {
        let n = conn.read(&mut buf[pos..]).await?;
        if n == 0 {
            return Err("unexpected eof while tunneling".into());
        }
        pos += n;

        let recvd = &buf[..pos];
        if recvd.starts_with(b"HTTP/1.1 200") || recvd.starts_with(b"HTTP/1.0 200") {
            if recvd.ends_with(b"\r\n\r\n") {
                return Ok(conn);
            }
            if pos == buf.len() {
                return Err("proxy headers too long for tunnel".into());
            }
            // else read more
        } else if recvd.starts_with(b"HTTP/1.1 407") {
            return Err("proxy authentication required".into());
        } else {
            return Err("unsuccessful tunnel".into());
        }
    }
}

// `Conn` from `reqwest`.
// https://github.com/seanmonstar/reqwest/blob/ab49de875ec2326abf25f52f54b249a28e43b69c/src/connect.rs#L589-L597
// Note: the `is_proxy` member means *is plain text HTTP proxy*.
// This tells hyper whether the URI should be written in
// * origin-form (`GET /just/a/path HTTP/1.1`), when `is_proxy == false`, or
// * absolute-form (`GET http://foo.bar/and/a/path HTTP/1.1`), otherwise.
#[pin_project]
pub struct Conn {
    #[pin]
    inner: BoxConn,
    is_proxy: bool,
}

impl Connection for Conn {
    fn connected(&self) -> Connected {
        self.inner.connected().proxy(self.is_proxy)
    }
}

impl AsyncRead for Conn {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.project();
        AsyncRead::poll_read(this.inner, cx, buf)
    }
}

impl AsyncWrite for Conn {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        AsyncWrite::poll_write(this.inner, cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[io::IoSlice<'_>],
    ) -> Poll<Result<usize, io::Error>> {
        let this = self.project();
        AsyncWrite::poll_write_vectored(this.inner, cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        AsyncWrite::poll_flush(this.inner, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), io::Error>> {
        let this = self.project();
        AsyncWrite::poll_shutdown(this.inner, cx)
    }
}

#[cfg(feature = "native-tls")]
mod native_tls_conn {
    use std::{
        io::{self, IoSlice},
        pin::Pin,
        task::{Context, Poll},
    };

    use hyper::client::connect::{Connected, Connection};
    use pin_project::pin_project;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio_native_tls::TlsStream;

    #[pin_project]
    pub(super) struct NativeTlsConn<T> {
        #[pin]
        pub(super) inner: TlsStream<T>,
    }

    impl<T: Connection + AsyncRead + AsyncWrite + Unpin> Connection for NativeTlsConn<T> {
        fn connected(&self) -> Connected {
            self.inner.get_ref().get_ref().get_ref().connected()
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for NativeTlsConn<T> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<tokio::io::Result<()>> {
            let this = self.project();
            AsyncRead::poll_read(this.inner, cx, buf)
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for NativeTlsConn<T> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write(this.inner, cx, buf)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize, io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write_vectored(this.inner, cx, bufs)
        }

        fn is_write_vectored(&self) -> bool {
            self.inner.is_write_vectored()
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_flush(this.inner, cx)
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_shutdown(this.inner, cx)
        }
    }
}

#[cfg(feature = "rustls-tls")]
mod rustls_tls_conn {
    use std::{
        io::{self, IoSlice},
        pin::Pin,
        task::{Context, Poll},
    };

    use hyper::client::connect::{Connected, Connection};
    use pin_project::pin_project;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
    use tokio_rustls::client::TlsStream;

    #[pin_project]
    pub(super) struct RustlsTlsConn<T> {
        #[pin]
        pub(super) inner: TlsStream<T>,
    }

    impl<T: Connection + AsyncRead + AsyncWrite + Unpin> Connection for RustlsTlsConn<T> {
        fn connected(&self) -> Connected {
            if self.inner.get_ref().1.alpn_protocol() == Some(b"h2") {
                self.inner.get_ref().0.connected().negotiated_h2()
            } else {
                self.inner.get_ref().0.connected()
            }
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncRead for RustlsTlsConn<T> {
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<tokio::io::Result<()>> {
            let this = self.project();
            AsyncRead::poll_read(this.inner, cx, buf)
        }
    }

    impl<T: AsyncRead + AsyncWrite + Unpin> AsyncWrite for RustlsTlsConn<T> {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context,
            buf: &[u8],
        ) -> Poll<Result<usize, tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write(this.inner, cx, buf)
        }

        fn poll_write_vectored(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            bufs: &[IoSlice<'_>],
        ) -> Poll<Result<usize, io::Error>> {
            let this = self.project();
            AsyncWrite::poll_write_vectored(this.inner, cx, bufs)
        }

        fn is_write_vectored(&self) -> bool {
            self.inner.is_write_vectored()
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_flush(this.inner, cx)
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), tokio::io::Error>> {
            let this = self.project();
            AsyncWrite::poll_shutdown(this.inner, cx)
        }
    }
}
