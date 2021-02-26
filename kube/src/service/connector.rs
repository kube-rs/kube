pub use inner::https_connector;
#[cfg(any(feature = "proxy-native-tls", feature = "proxy-rustls-tls"))]
pub use inner::proxy_connector;

#[cfg(feature = "rustls-tls")] pub use inner::tls_config;
#[cfg(feature = "native-tls")] pub use inner::tls_connector;

#[cfg(feature = "native-tls")]
mod inner {
    use hyper::client::HttpConnector;
    use hyper_tls::HttpsConnector;
    use tokio_native_tls::{
        native_tls::{self, Certificate, Identity},
        TlsConnector as AsyncTlsConnector,
    };

    #[cfg(feature = "proxy-native-tls")]
    use hyper_proxy::{Intercept, Proxy, ProxyConnector};

    use crate::{Error, Result};

    pub fn tls_connector(
        identity: Option<(Vec<u8>, String)>,
        root_cert: Option<Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<native_tls::TlsConnector> {
        let mut builder = native_tls::TlsConnector::builder();
        if let Some((pem, identity_password)) = identity.as_ref() {
            let identity = pkcs12_from_pem(pem, identity_password)?;
            builder.identity(
                Identity::from_pkcs12(&identity, identity_password)
                    .map_err(|e| Error::SslError(format!("{}", e)))?,
            );
        }

        if let Some(ders) = root_cert {
            for der in ders {
                builder.add_root_certificate(
                    Certificate::from_der(&der).map_err(|e| Error::SslError(format!("{}", e)))?,
                );
            }
        }

        if accept_invalid {
            builder.danger_accept_invalid_certs(accept_invalid);
        }

        builder.build().map_err(|e| Error::SslError(format!("{}", e)))
    }

    pub fn https_connector(connector: native_tls::TlsConnector) -> HttpsConnector<HttpConnector> {
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        HttpsConnector::from((http, AsyncTlsConnector::from(connector)))
    }

    #[cfg(feature = "proxy-native-tls")]
    pub fn proxy_connector(
        connector: native_tls::TlsConnector,
        proxy_url: Option<http::uri::Uri>,
    ) -> hyper_proxy::ProxyConnector<HttpsConnector<HttpConnector>> {
        let mut proxy = ProxyConnector::unsecured(https_connector(connector.clone()));
        if let Some(proxy_url) = proxy_url {
            proxy.add_proxy(Proxy::new(Intercept::All, proxy_url));
        }
        proxy.set_tls(Some(connector));
        proxy
    }

    fn pkcs12_from_pem(pem: &[u8], password: &str) -> Result<Vec<u8>> {
        use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
        let x509 = X509::from_pem(&pem)?;
        let pkey = PKey::private_key_from_pem(&pem)?;
        let p12 = Pkcs12::builder().build(password, "kubeconfig", &pkey, &x509)?;
        let der = p12.to_der()?;
        Ok(der)
    }
}

#[cfg(feature = "rustls-tls")]
mod inner {
    use std::sync::Arc;

    use hyper::client::HttpConnector;
    use hyper_rustls::HttpsConnector;
    use tokio_rustls::{
        rustls::{self, Certificate, ClientConfig, ServerCertVerified, ServerCertVerifier},
        webpki::DNSNameRef,
    };

    #[cfg(feature = "proxy-rustls-tls")]
    use hyper_proxy::{Intercept, Proxy, ProxyConnector};

    use crate::{Error, Result};

    pub fn tls_config(
        identity: Option<(Vec<u8>, String)>,
        root_cert: Option<Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<ClientConfig> {
        use rustls::internal::pemfile;
        use std::io::Cursor;

        // Based on code from `reqwest`
        let mut client_config = ClientConfig::new();
        if let Some((buf, _)) = identity.as_ref() {
            let (key, certs) = {
                let mut pem = Cursor::new(buf);
                let certs = pemfile::certs(&mut pem)
                    .map_err(|_| Error::SslError("No valid certificate was found".into()))?;
                pem.set_position(0);

                let mut sk = pemfile::pkcs8_private_keys(&mut pem)
                    .and_then(|pkcs8_keys| {
                        if pkcs8_keys.is_empty() {
                            Err(())
                        } else {
                            Ok(pkcs8_keys)
                        }
                    })
                    .or_else(|_| {
                        pem.set_position(0);
                        pemfile::rsa_private_keys(&mut pem)
                    })
                    .map_err(|_| Error::SslError("No valid private key was found".into()))?;

                if let (Some(sk), false) = (sk.pop(), certs.is_empty()) {
                    (sk, certs)
                } else {
                    return Err(Error::SslError("private key or certificate not found".into()));
                }
            };

            client_config
                .set_single_client_cert(certs, key)
                .map_err(|e| Error::SslError(format!("{}", e)))?;
        }

        if let Some(ders) = root_cert {
            for der in ders {
                client_config
                    .root_store
                    .add(&Certificate(der))
                    .map_err(|e| Error::SslError(format!("{}", e)))?;
            }
        }

        if accept_invalid {
            client_config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
        }

        Ok(client_config)
    }

    pub fn https_connector(tls_config: Arc<ClientConfig>) -> HttpsConnector<HttpConnector> {
        let mut http = HttpConnector::new();
        http.enforce_http(false);
        HttpsConnector::from((http, tls_config))
    }

    #[cfg(feature = "proxy-rustls-tls")]
    pub fn proxy_connector(
        tls_config: Arc<ClientConfig>,
        proxy_url: Option<http::uri::Uri>,
    ) -> hyper_proxy::ProxyConnector<HttpsConnector<HttpConnector>> {
        let mut connector = ProxyConnector::unsecured(https_connector(tls_config.clone()));
        if let Some(proxy_url) = proxy_url {
            connector.add_proxy(Proxy::new(Intercept::All, proxy_url));
        }
        let tls = tokio_rustls::TlsConnector::from(tls_config);
        connector.set_tls(Some(tls));
        connector
    }

    struct NoCertificateVerification {}

    impl ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _roots: &rustls::RootCertStore,
            _presented_certs: &[rustls::Certificate],
            _dns_name: DNSNameRef<'_>,
            _ocsp: &[u8],
        ) -> Result<ServerCertVerified, rustls::TLSError> {
            Ok(ServerCertVerified::assertion())
        }
    }
}
