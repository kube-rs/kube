use std::sync::Arc;

use http::{header::HeaderName, HeaderValue};
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
    /// # use kube::{client::ConfigExt, Config};
    /// let config = Config::infer().await?;
    /// let https = config.rustls_https_connector()?;
    /// let hyper_client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(https);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

    /// Create [`rustls::ClientConfig`] based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper::client::HttpConnector;
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
    fn openssl_https_connector(&self) -> Result<hyper_openssl::HttpsConnector<hyper::client::HttpConnector>>;

    /// Create [`hyper_openssl::HttpsConnector`] based on config and `connector`.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper::client::HttpConnector;
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
    fn openssl_https_connector_with_connector(
        &self,
        connector: hyper::client::HttpConnector,
    ) -> Result<hyper_openssl::HttpsConnector<hyper::client::HttpConnector>>;

    /// Create [`openssl::ssl::SslConnectorBuilder`] based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper::client::HttpConnector;
    /// # use kube::{client::ConfigExt, Client, Config};
    /// let config = Config::infer().await?;
    /// let https = {
    ///     let mut http = HttpConnector::new();
    ///     http.enforce_http(false);
    ///     let ssl = config.openssl_ssl_connector_builder()?;
    ///     hyper_openssl::HttpsConnector::with_connector(http, ssl)?
    /// };
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
    #[cfg(feature = "openssl-tls")]
    fn openssl_ssl_connector_builder(&self) -> Result<openssl::ssl::SslConnectorBuilder>;
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
            Auth::Basic(user, pass) => Some(AuthLayer(Either::A(
                AddAuthorizationLayer::basic(&user, pass.expose_secret()).as_sensitive(true),
            ))),
            Auth::Bearer(token) => Some(AuthLayer(Either::A(
                AddAuthorizationLayer::bearer(token.expose_secret()).as_sensitive(true),
            ))),
            Auth::RefreshableToken(refreshable) => {
                Some(AuthLayer(Either::B(AsyncFilterLayer::new(refreshable))))
            }
        })
    }

    fn extra_headers_layer(&self) -> Result<ExtraHeadersLayer> {
        let mut headers = Vec::new();
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
        tls::rustls_tls::rustls_client_config(
            self.identity_pem().as_deref(),
            self.root_cert.as_deref(),
            self.accept_invalid_certs,
        )
        .map_err(Error::RustlsTls)
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>> {
        let rustls_config = std::sync::Arc::new(self.rustls_client_config()?);
        let mut http = hyper::client::HttpConnector::new();
        http.enforce_http(false);
        Ok(hyper_rustls::HttpsConnector::from((http, rustls_config)))
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_ssl_connector_builder(&self) -> Result<openssl::ssl::SslConnectorBuilder> {
        tls::openssl_tls::ssl_connector_builder(self.identity_pem().as_ref(), self.root_cert.as_ref())
            .map_err(|e| Error::OpensslTls(tls::openssl_tls::Error::CreateSslConnector(e)))
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector(&self) -> Result<hyper_openssl::HttpsConnector<hyper::client::HttpConnector>> {
        let mut connector = hyper::client::HttpConnector::new();
        connector.enforce_http(false);
        self.openssl_https_connector_with_connector(connector)
    }

    #[cfg(feature = "openssl-tls")]
    fn openssl_https_connector_with_connector(
        &self,
        connector: hyper::client::HttpConnector,
    ) -> Result<hyper_openssl::HttpsConnector<hyper::client::HttpConnector>> {
        let mut https =
            hyper_openssl::HttpsConnector::with_connector(connector, self.openssl_ssl_connector_builder()?)
                .map_err(|e| Error::OpensslTls(tls::openssl_tls::Error::CreateHttpsConnector(e)))?;
        if self.accept_invalid_certs {
            https.set_callback(|ssl, _uri| {
                ssl.set_verify(openssl::ssl::SslVerifyMode::NONE);
                Ok(())
            });
        }
        Ok(https)
    }
}
