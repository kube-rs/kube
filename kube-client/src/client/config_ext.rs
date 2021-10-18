use std::convert::TryFrom;

use tower::util::Either;

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))] use super::tls;
use super::{
    auth::Auth,
    middleware::{AddAuthorizationLayer, AuthLayer, BaseUriLayer, RefreshTokenLayer},
};
use crate::{Config, Result};

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

    /// Create [`hyper_tls::HttpsConnector`] based on config.
    ///
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use kube::{client::ConfigExt, Config};
    /// let config = Config::infer().await?;
    /// let https = config.native_tls_https_connector()?;
    /// let hyper_client: hyper::Client<_, hyper::Body> = hyper::Client::builder().build(https);
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    fn native_tls_https_connector(&self) -> Result<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

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

    /// Create [`native_tls::TlsConnector`](tokio_native_tls::native_tls::TlsConnector) based on config.
    /// # Example
    ///
    /// ```rust
    /// # async fn doc() -> Result<(), Box<dyn std::error::Error>> {
    /// # use hyper::client::HttpConnector;
    /// # use kube::{client::ConfigExt, Client, Config};
    /// let config = Config::infer().await?;
    /// let https = {
    ///     let tls = tokio_native_tls::TlsConnector::from(
    ///         config.native_tls_connector()?
    ///     );
    ///     let mut http = HttpConnector::new();
    ///     http.enforce_http(false);
    ///     hyper_tls::HttpsConnector::from((http, tls))
    /// };
    /// # Ok(())
    /// # }
    /// ```
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    fn native_tls_connector(&self) -> Result<tokio_native_tls::native_tls::TlsConnector>;

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
        Ok(match Auth::try_from(&self.auth_info)? {
            Auth::None => None,
            Auth::Basic(user, pass) => Some(AuthLayer(Either::A(
                AddAuthorizationLayer::basic(&user, &pass).as_sensitive(true),
            ))),
            Auth::Bearer(token) => Some(AuthLayer(Either::A(
                AddAuthorizationLayer::bearer(&token).as_sensitive(true),
            ))),
            Auth::RefreshableToken(r) => Some(AuthLayer(Either::B(RefreshTokenLayer::new(r)))),
        })
    }

    #[cfg(feature = "native-tls")]
    fn native_tls_connector(&self) -> Result<tokio_native_tls::native_tls::TlsConnector> {
        tls::native_tls::native_tls_connector(
            self.identity_pem.as_ref(),
            self.root_cert.as_ref(),
            self.accept_invalid_certs,
        )
    }

    #[cfg(feature = "native-tls")]
    fn native_tls_https_connector(&self) -> Result<hyper_tls::HttpsConnector<hyper::client::HttpConnector>> {
        let tls = tokio_native_tls::TlsConnector::from(self.native_tls_connector()?);
        let mut http = hyper::client::HttpConnector::new();
        http.enforce_http(false);
        Ok(hyper_tls::HttpsConnector::from((http, tls)))
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_client_config(&self) -> Result<rustls::ClientConfig> {
        tls::rustls_tls::rustls_client_config(
            self.identity_pem.as_ref(),
            self.root_cert.as_ref(),
            self.accept_invalid_certs,
        )
    }

    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>> {
        let rustls_config = std::sync::Arc::new(self.rustls_client_config()?);
        let mut http = hyper::client::HttpConnector::new();
        http.enforce_http(false);
        Ok(hyper_rustls::HttpsConnector::from((http, rustls_config)))
    }
}
