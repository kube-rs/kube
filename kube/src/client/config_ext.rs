use std::convert::TryFrom;

use tower::util::Either;

#[cfg(any(feature = "native-tls", feature = "rustls-tls"))] use super::tls;
use super::{
    auth::{AddAuthorizationLayer, RefreshingTokenLayer},
    Auth, SetBaseUriLayer,
};
use crate::{Config, Result};

/// Extensions to `Config` for `Client`.
///
/// This trait is sealed and cannot be implemented.
pub trait ConfigExt: private::Sealed {
    /// Layer to set the base URI of requests to the configured server.
    fn base_uri_layer(&self) -> SetBaseUriLayer;

    /// Create `native_tls::TlsConnector`
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    fn native_tls_connector(&self) -> Result<tokio_native_tls::native_tls::TlsConnector>;

    /// Create `hyper_tls::HttpsConnector`
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[cfg(feature = "native-tls")]
    fn native_tls_https_connector(&self) -> Result<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

    /// Create `rustls::ClientConfig`
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_client_config(&self) -> Result<rustls::ClientConfig>;

    /// Create `hyper_rustls::HttpsConnector`
    #[cfg_attr(docsrs, doc(cfg(feature = "rustls-tls")))]
    #[cfg(feature = "rustls-tls")]
    fn rustls_https_connector(&self) -> Result<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>;

    // TODO Try reducing exported types to minimize API surface before making this public.
    #[doc(hidden)]
    /// Optional layer to set up `Authorization` header depending on the config.
    ///
    /// Users are not allowed to call this for now.
    fn auth_layer(&self) -> Result<Option<Either<AddAuthorizationLayer, RefreshingTokenLayer>>>;
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Config {}
}

impl ConfigExt for Config {
    fn base_uri_layer(&self) -> SetBaseUriLayer {
        SetBaseUriLayer::new(self.cluster_url.clone())
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

    fn auth_layer(&self) -> Result<Option<Either<AddAuthorizationLayer, RefreshingTokenLayer>>> {
        Ok(match Auth::try_from(&self.auth_info)? {
            Auth::None => None,
            Auth::Basic(user, pass) => Some(Either::A(AddAuthorizationLayer::basic(&user, &pass))),
            Auth::Bearer(token) => Some(Either::A(AddAuthorizationLayer::bearer(&token))),
            Auth::RefreshableToken(r) => Some(Either::B(RefreshingTokenLayer::new(r))),
        })
    }
}
