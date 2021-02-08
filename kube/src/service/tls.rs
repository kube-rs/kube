// Create `HttpsConnector` from `Config`.
// - hyper_tls::HttpsConnector from (hyper::client::HttpConnector, tokio_native_tls::TlsConnector)
// - hyper_rustls::HttpsConnector from (hyper::client::HttpConnector, Arc<rustls::ClientConfig>)

pub use connector::HttpsConnector;

#[cfg(feature = "native-tls")]
mod connector {
    use std::convert::{TryFrom, TryInto};

    use hyper::client::HttpConnector;
    use tokio_native_tls::native_tls::{Certificate, Identity, TlsConnector};

    use crate::{Config, Error, Result};

    pub use hyper_tls::HttpsConnector;
    use tokio_native_tls::TlsConnector as AsyncTlsConnector;

    impl TryFrom<Config> for HttpsConnector<HttpConnector> {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            let tls: AsyncTlsConnector = config.try_into()?;
            Ok(HttpsConnector::from((http, tls)))
        }
    }

    impl TryFrom<Config> for AsyncTlsConnector {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            let mut builder = TlsConnector::builder();
            if let Some((pem, identity_password)) = config.identity.as_ref() {
                let identity = pkcs12_from_pem(pem, identity_password)?;
                builder.identity(
                    Identity::from_pkcs12(&identity, identity_password)
                        .map_err(|e| Error::SslError(format!("{}", e)))?,
                );
            }

            if let Some(ders) = config.root_cert {
                for der in ders {
                    builder.add_root_certificate(
                        Certificate::from_der(&der).map_err(|e| Error::SslError(format!("{}", e)))?,
                    );
                }
            }

            if config.accept_invalid_certs {
                builder.danger_accept_invalid_certs(config.accept_invalid_certs);
            }

            let connector = builder.build().map_err(|e| Error::SslError(format!("{}", e)))?;
            Ok(AsyncTlsConnector::from(connector))
        }
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
mod connector {
    use std::{
        convert::{TryFrom, TryInto},
        sync::Arc,
    };

    use hyper::client::HttpConnector;
    use tokio_rustls::{
        rustls::{self, Certificate, ClientConfig, ServerCertVerified, ServerCertVerifier},
        webpki::DNSNameRef,
    };

    use crate::{config::Config, Error, Result};

    pub use hyper_rustls::HttpsConnector;

    impl TryFrom<Config> for HttpsConnector<HttpConnector> {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            let mut http = HttpConnector::new();
            http.enforce_http(false);
            let client_config: ClientConfig = config.try_into()?;
            let client_config = Arc::new(client_config);

            Ok(HttpsConnector::from((http, client_config)))
        }
    }

    impl TryFrom<Config> for ClientConfig {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            use rustls::internal::pemfile;
            use std::io::Cursor;

            // Based on code from `reqwest`
            let mut client_config = ClientConfig::new();
            if let Some((buf, _)) = config.identity.as_ref() {
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

            if let Some(ders) = config.root_cert {
                for der in ders {
                    client_config
                        .root_store
                        .add(&Certificate(der))
                        .map_err(|e| Error::SslError(format!("{}", e)))?;
                }
            }

            if config.accept_invalid_certs {
                client_config
                    .dangerous()
                    .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
            }

            Ok(client_config)
        }
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
