pub use tls::AsyncTlsConnector;

#[cfg(feature = "ws-native-tls")]
mod tls {
    use std::convert::TryFrom;

    use tokio_native_tls::native_tls::{Certificate, Identity, TlsConnector};
    pub use tokio_native_tls::TlsConnector as AsyncTlsConnector;

    use crate::{config::Config, Error, Result};

    impl TryFrom<Config> for AsyncTlsConnector {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            let mut builder = TlsConnector::builder();
            if let Some((identity, identity_password)) = config.identity.as_ref() {
                builder.identity(
                    Identity::from_pkcs12(identity, identity_password)
                        .map_err(|e| Error::SslError(format!("{}", e)))?,
                );
            }
            if let Some(ders) = config.root_cert {
                for der in ders {
                    builder.add_root_certificate(
                        Certificate::from_der(&der.0).map_err(|e| Error::SslError(format!("{}", e)))?,
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
}

#[cfg(feature = "ws-rustls-tls")]
mod tls {
    use std::{convert::TryFrom, sync::Arc};

    pub use tokio_rustls::TlsConnector as AsyncTlsConnector;
    use tokio_rustls::{
        rustls::{self, Certificate, ClientConfig},
        webpki,
    };

    use crate::{config::Config, Error, Result};

    impl TryFrom<Config> for AsyncTlsConnector {
        type Error = Error;

        fn try_from(config: Config) -> Result<Self> {
            use rustls::internal::pemfile;
            use std::io::Cursor;

            let mut client_config = ClientConfig::new();
            // This is based on how `reqwest` does
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
                        .add(&Certificate(der.0))
                        .map_err(|e| Error::SslError(format!("{}", e)))?;
                }
            }

            if config.accept_invalid_certs {
                client_config
                    .dangerous()
                    .set_certificate_verifier(Arc::new(NoCertificateVerification {}));
            }

            Ok(AsyncTlsConnector::from(Arc::new(client_config)))
        }
    }

    struct NoCertificateVerification {}

    impl rustls::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _roots: &rustls::RootCertStore,
            _presented_certs: &[rustls::Certificate],
            _dns_name: webpki::DNSNameRef<'_>,
            _ocsp: &[u8],
        ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            Ok(rustls::ServerCertVerified::assertion())
        }
    }
}
