use std::sync::Arc;

use http::{HeaderValue, header::HeaderName};
#[cfg(feature = "openssl-tls")] use hyper::rt::{Read, Write};
use hyper_util::client::legacy::connect::HttpConnector;
use jiff::Timestamp;
use secrecy::ExposeSecret;
use tower::{filter::AsyncFilterLayer, util::Either};

#[cfg(any(feature = "rustls-tls", feature = "openssl-tls"))] use super::tls;
use super::{
    auth::Auth,
    middleware::{AddAuthorizationLayer, AuthLayer, BaseUriLayer, ExtraHeadersLayer},
};
use crate::{Config, Error, Result};

/// Extensions to [`Config`](crate::Config) for custom [`Client`](crate::Client).
///
/// See [`Client::new`](crate::Client::new) for an example.
///
/// This trait is sealed and cannot be implemented.
pub trait ConfigExt: private::Sealed {
    /// Layer to set the base URI of requests to the configured server.
    fn base_uri_layer(&self) -> BaseUriLayer;

    /// Optional layer to set up `Authorization` header depending on the config.
    fn auth_layer(&self) -> Result<Option<AuthLayer>>;

    /// Layer to add non-authn HTTP headers depending on the config.
    fn extra_headers_layer(&self) -> Result<ExtraHeadersLayer>;

    /// Create [`hyper_rustls::HttpsConnector`] based on config.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use kube::{client::{Body, ConfigExt}, Config};
    /// # use hyper_util::rt::TokioExecutor;
    /// let config = Config::infer().await?;
    /// let https = config.rustls_https_connector()?;
    /// let hyper_client: hyper_util::client::legacy::Client<_, Body> = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<HttpConnector>>;

    /// Create [`hyper_rustls::HttpsConnector`] based on config and `connector`.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use kube::{client::{Body, ConfigExt}, Config};
    /// # use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};
    /// let config = Config::infer().await?;
    /// let mut connector = HttpConnector::new();
    /// connector.enforce_http(false);
    /// let https = config.rustls_https_connector_with_connector(connector)?;
    /// let hyper_client: hyper_util::client::legacy::Client<_, Body> = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector_with_connector<H>(
        &self,
        connector: H,
    ) -> Result<hyper_rustls::HttpsConnector<H>>;

    /// Create [`rustls::ClientConfig`] based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper_util::client::legacy::connect::HttpConnector;
    /// # use kube::{client::ConfigExt, Config};
    /// let config = Config::infer().await?;
    /// let https = {
    ///     let rustls_config = std::sync::Arc::new(config.rustls_client_config()?);
    ///     let mut http = HttpConnector::new();
    ///     http.enforce_http(false);
    ///     hyper_rustls::HttpsConnector::from((http, rustls_config))
    /// };
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_client_config(&self) -> Result<rustls::ClientConfig>;

    /// Create [`hyper_openssl::HttpsConnector`] based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use kube::{client::ConfigExt, Config};
    /// let config = Config::infer().await?;
    /// let https = config.openssl_https_connector()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector(&self)
    -> Result<hyper_openssl::client::legacy::HttpsConnector<HttpConnector>>;

    /// Create [`hyper_openssl::HttpsConnector`] based on config and `connector`.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper_util::client::legacy::connect::HttpConnector;
    /// # use kube::{client::ConfigExt, Config};
    /// let mut http = HttpConnector::new();
    /// http.enforce_http(false);
    /// let config = Config::infer().await?;
    /// let https = config.openssl_https_connector_with_connector(http)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector_with_connector<H>(
        &self,
        connector: H,
    ) -> Result<hyper_openssl::client::legacy::HttpsConnector<H>>
    where
        H: tower::Service<http::Uri> + Send,
        H::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        H::Future: Send + 'static,
        H::Response: Read + Write + hyper_util::client::legacy::connect::Connection + Unpin;

    /// Create [`openssl::ssl::SslConnectorBuilder`] based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper_util::client::legacy::connect::HttpConnector;
    /// # use kube::{client::ConfigExt, Client, Config};
    /// let config = Config::infer().await?;
    /// let https = {
    ///     let mut http = HttpConnector::new();
    ///     http.enforce_http(false);
    ///     let ssl = config.openssl_ssl_connector_builder()?;
    ///     hyper_openssl::client::legacy::HttpsConnector::with_connector(http, ssl)?
    /// };
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
    #[cfg(feature = "openssl-tls")]
    fn openssl_ssl_connector_builder(&self) -> Result<openssl::ssl::SslConnectorBuilder>;
}

#[cfg(all(test, feature = "openssl-tls"))]
mod openssl_tls_server_name_tests {
    use std::{
        net::TcpListener,
        sync::{Arc, Mutex},
    };

    use openssl::{
        asn1::Asn1Time,
        hash::MessageDigest,
        pkey::{PKey, Private},
        rsa::Rsa,
        ssl::{NameType, SslAcceptor, SslMethod},
        x509::{
            extension::{BasicConstraints, SubjectAlternativeName},
            X509NameBuilder, X509,
        },
    };
    use tower::ServiceExt as _;

    use super::*;

    // Self-signed cert whose only SAN is `dns`, so it does not match the 127.0.0.1 we connect to.
    // Verification then only passes if the verify-host comes from tls_server_name.
    fn self_signed_cert(dns: &str) -> (X509, PKey<Private>) {
        let pkey = PKey::from_rsa(Rsa::generate(2048).unwrap()).unwrap();

        let mut name = X509NameBuilder::new().unwrap();
        name.append_entry_by_text("CN", dns).unwrap();
        let name = name.build();

        let mut builder = X509::builder().unwrap();
        builder.set_version(2).unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(&name).unwrap();
        builder.set_pubkey(&pkey).unwrap();
        builder.set_not_before(&Asn1Time::days_from_now(0).unwrap()).unwrap();
        builder.set_not_after(&Asn1Time::days_from_now(1).unwrap()).unwrap();
        builder
            .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
            .unwrap();
        let san = SubjectAlternativeName::new()
            .dns(dns)
            .build(&builder.x509v3_context(None, None))
            .unwrap();
        builder.append_extension(san).unwrap();
        builder.sign(&pkey, MessageDigest::sha256()).unwrap();

        (builder.build(), pkey)
    }

    // Localhost TLS server that records the SNI from the one connection it accepts.
    fn spawn_tls_server(cert: X509, key: PKey<Private>) -> (u16, Arc<Mutex<Option<String>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        let captured_in_cb = captured.clone();
        let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
        acceptor.set_private_key(&key).unwrap();
        acceptor.set_certificate(&cert).unwrap();
        acceptor.set_servername_callback(move |ssl, _alert| {
            *captured_in_cb.lock().unwrap() = ssl.servername(NameType::HOST_NAME).map(str::to_owned);
            Ok(())
        });
        let acceptor = acceptor.build();

        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                // SNI is captured during the handshake; we don't care whether it then completes.
                let _ = acceptor.accept(stream);
            }
        });

        (port, captured)
    }

    fn config_for(port: u16, ca: &X509, tls_server_name: Option<&str>) -> Config {
        let mut config = Config::new(format!("https://127.0.0.1:{port}").parse().unwrap());
        config.root_cert = Some(vec![ca.to_der().unwrap()]);
        config.tls_server_name = tls_server_name.map(str::to_owned);
        config
    }

    fn connector_for(config: &Config) -> hyper_openssl::client::legacy::HttpsConnector<HttpConnector> {
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        config.openssl_https_connector_with_connector(http).unwrap()
    }

    // tls_server_name set: SNI carries it (not the 127.0.0.1 host) and verification targets it,
    // so the handshake against the SAN-only cert succeeds.
    #[tokio::test]
    async fn tls_server_name_drives_sni_and_verification() {
        let server_name = "kubernetes.example.com";
        let (cert, key) = self_signed_cert(server_name);
        let (port, captured_sni) = spawn_tls_server(cert.clone(), key);

        let config = config_for(port, &cert, Some(server_name));
        let uri: http::Uri = config.cluster_url.clone();

        connector_for(&config)
            .oneshot(uri)
            .await
            .expect("handshake should succeed when verification targets tls_server_name");

        assert_eq!(
            captured_sni.lock().unwrap().as_deref(),
            Some(server_name),
            "ClientHello SNI must equal tls_server_name, not the connection host"
        );
    }

    // Control: without tls_server_name, verification falls back to the 127.0.0.1 host, which the
    // SAN-only cert doesn't match, so the handshake fails. Confirms the pass above isn't just lax
    // verification.
    #[tokio::test]
    async fn without_tls_server_name_verification_uses_connection_host() {
        let server_name = "kubernetes.example.com";
        let (cert, key) = self_signed_cert(server_name);
        let (port, _captured_sni) = spawn_tls_server(cert.clone(), key);

        let config = config_for(port, &cert, None);
        let uri: http::Uri = config.cluster_url.clone();

        let result = connector_for(&config).oneshot(uri).await;
        assert!(
            result.is_err(),
            "handshake must fail when the cert does not match the connection host"
        );
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Config {}
}

impl ConfigExt for Config {
    fn base_uri_layer(&self) -> BaseUriLayer {
        BaseUriLayer::new(self.cluster_url.clone())
    }

    fn auth_layer(&self) -> Result<Option<AuthLayer>> {
        Ok(match Auth::try_from(&self.auth_info).map_err(Error::Auth)? {
            Auth::None => None,
            Auth::Basic(user, pass) => Some(AuthLayer(Either::Left(
                AddAuthorizationLayer::basic(&user, pass.expose_secret()).as_sensitive(true),
            ))),
            Auth::Bearer(token) => Some(AuthLayer(Either::Left(
                AddAuthorizationLayer::bearer(token.expose_secret()).as_sensitive(true),
            ))),
            Auth::RefreshableToken(refreshable) => {
                Some(AuthLayer(Either::Right(AsyncFilterLayer::new(refreshable))))
            }
            Auth::Certificate(_client_certificate_data, _client_key_data, _) => None,
        })
    }

    fn extra_headers_layer(&self) -> Result<ExtraHeadersLayer> {
        let mut headers = self.headers.clone();
        if let Some(impersonate_user) = &self.auth_info.impersonate {
            headers.push((
                HeaderName::from_static("impersonate-user"),
                HeaderValue::from_str(impersonate_user)
                    .map_err(http::Error::from)
                    .map_err(Error::HttpError)?,
            ));
        }
        if let Some(impersonate_groups) = &self.auth_info.impersonate_groups {
            for group in impersonate_groups {
                headers.push((
                    HeaderName::from_static("impersonate-group"),
                    HeaderValue::from_str(group)
                        .map_err(http::Error::from)
                        .map_err(Error::HttpError)?,
                ));
            }
        }
        Ok(ExtraHeadersLayer {
            headers: Arc::new(headers),
        })
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_client_config(&self) -> Result<rustls::ClientConfig> {
        let identity = match self.exec_identity_pem().0 {
            Some(identity) => Some(identity),
            None => self.identity_pem()?,
        };
        let mut config = tls::rustls_tls::rustls_client_config(
            identity.as_deref(),
            self.root_cert.as_deref(),
            self.accept_invalid_certs,
        )
        .map_err(Error::RustlsTls)?;

        // When a CA file path is available (in-cluster), install a verifier
        // that re-reads it periodically to survive CA rotation. `root_cert`
        // bytes are still passed above so the builder typestate is satisfied,
        // but verification is handed over here.
        if !self.accept_invalid_certs
            && let Some(path) = &self.root_cert_file
        {
            let verifier =
                tls::rustls_tls::ReloadingVerifier::new(path.clone()).map_err(Error::RustlsTls)?;
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(verifier));
        }
        Ok(config)
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<HttpConnector>> {
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);
        self.rustls_https_connector_with_connector(connector)
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector_with_connector<H>(
        &self,
        connector: H,
    ) -> Result<hyper_rustls::HttpsConnector<H>> {
        use hyper_rustls::FixedServerNameResolver;

        use crate::client::tls::rustls_tls;

        let rustls_config = self.rustls_client_config()?;
        let mut builder = hyper_rustls::HttpsConnectorBuilder::new()
            .with_tls_config(rustls_config)
            .https_or_http();
        if let Some(tsn) = self.tls_server_name.as_ref() {
            builder = builder.with_server_name_resolver(FixedServerNameResolver::new(
                tsn.clone()
                    .try_into()
                    .map_err(rustls_tls::Error::InvalidServerName)
                    .map_err(Error::RustlsTls)?,
            ));
        }
        Ok(builder.enable_http1().wrap_connector(connector))
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_ssl_connector_builder(&self) -> Result<openssl::ssl::SslConnectorBuilder> {
        let identity = match self.exec_identity_pem().0 {
            Some(identity) => Some(identity),
            None => self.identity_pem()?,
        };

        // tls_server_name has no hook on the builder; it is applied per-connection in
        // openssl_https_connector_with_connector instead.
        tls::openssl_tls::ssl_connector_builder(identity.as_ref(), self.root_cert.as_ref())
            .map_err(|e| Error::OpensslTls(tls::openssl_tls::Error::CreateSslConnector(e)))
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector(
        &self,
    ) -> Result<hyper_openssl::client::legacy::HttpsConnector<HttpConnector>> {
        let mut connector = HttpConnector::new();
        connector.enforce_http(false);
        self.openssl_https_connector_with_connector(connector)
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector_with_connector<H>(
        &self,
        connector: H,
    ) -> Result<hyper_openssl::client::legacy::HttpsConnector<H>>
    where
        H: tower::Service<http::Uri> + Send,
        H::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
        H::Future: Send + 'static,
        H::Response: Read + Write + hyper_util::client::legacy::connect::Connection + Unpin,
    {
        let mut https = hyper_openssl::client::legacy::HttpsConnector::with_connector(
            connector,
            self.openssl_ssl_connector_builder()?,
        )
        .map_err(|e| Error::OpensslTls(tls::openssl_tls::Error::CreateHttpsConnector(e)))?;
        // OpenSSL has no server-name-resolver hook, so unlike rustls we apply tls_server_name in
        // the per-connection callback (which already exists for accept_invalid_certs).
        let accept_invalid_certs = self.accept_invalid_certs;
        let tls_server_name = self.tls_server_name.clone();
        if accept_invalid_certs || tls_server_name.is_some() {
            https.set_callback(move |ssl, _uri| {
                if accept_invalid_certs {
                    ssl.set_verify(openssl::ssl::SslVerifyMode::NONE);
                }
                if let Some(name) = &tls_server_name {
                    use std::net::IpAddr;

                    use openssl::x509::verify::X509CheckFlags;
                    // into_ssl(host) runs after this callback and would otherwise set SNI and the
                    // verify host from the URI host. Disable both so it keeps our values.
                    ssl.set_use_server_name_indication(false);
                    ssl.set_verify_hostname(false);
                    // SNI is not sent for IP literals.
                    if name.parse::<IpAddr>().is_err() {
                        ssl.set_hostname(name)?;
                    }
                    let param = ssl.param_mut();
                    param.set_hostflags(X509CheckFlags::NO_PARTIAL_WILDCARDS);
                    match name.parse::<IpAddr>() {
                        Ok(ip) => param.set_ip(ip)?,
                        Err(_) => param.set_host(name)?,
                    }
                }
                Ok(())
            });
        }
        Ok(https)
    }
}

impl Config {
    // This is necessary to retrieve an identity when an exec plugin
    // returns a client certificate and key instead of a token.
    // This has be to be checked on TLS configuration vs tokens
    // which can be added in as an AuthLayer.
    pub(crate) fn exec_identity_pem(&self) -> (Option<Vec<u8>>, Option<Timestamp>) {
        match Auth::try_from(&self.auth_info) {
            Ok(Auth::Certificate(client_certificate_data, client_key_data, expiration)) => {
                const NEW_LINE: u8 = b'\n';

                let mut buffer = client_key_data.expose_secret().as_bytes().to_vec();
                buffer.push(NEW_LINE);
                buffer.extend_from_slice(client_certificate_data.as_bytes());
                buffer.push(NEW_LINE);
                (Some(buffer), expiration)
            }
            _ => (None, None),
        }
    }
}
