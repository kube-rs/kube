use crate::Result;

use super::Config;

impl Config {
    /// Create `native_tls::TlsConnector`
    #[cfg(feature = "native-tls")]
    pub fn native_tls_connector(&self) -> Result<tokio_native_tls::native_tls::TlsConnector> {
        self::native_tls::native_tls_connector(
            self.identity_pem.as_ref(),
            self.root_cert.as_ref(),
            self.accept_invalid_certs,
        )
    }

    /// Create `rustls::ClientConfig`
    #[cfg(feature = "rustls-tls")]
    pub fn rustls_tls_client_config(&self) -> Result<rustls::ClientConfig> {
        self::rustls_tls::rustls_client_config(
            self.identity_pem.as_ref(),
            self.root_cert.as_ref(),
            self.accept_invalid_certs,
        )
    }
}


#[cfg(feature = "native-tls")]
mod native_tls {
    use tokio_native_tls::native_tls::{Certificate, Identity, TlsConnector};

    use crate::{Error, Result};

    const IDENTITY_PASSWORD: &str = " ";

    /// Create `native_tls::TlsConnector`.
    pub fn native_tls_connector(
        identity_pem: Option<&Vec<u8>>,
        root_cert: Option<&Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<TlsConnector> {
        let mut builder = TlsConnector::builder();
        if let Some(pem) = identity_pem {
            let identity = pkcs12_from_pem(pem, IDENTITY_PASSWORD)?;
            builder.identity(
                Identity::from_pkcs12(&identity, IDENTITY_PASSWORD)
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
            builder.danger_accept_invalid_certs(true);
        }

        let connector = builder.build().map_err(|e| Error::SslError(format!("{}", e)))?;
        Ok(connector)
    }

    // TODO Replace this with pure Rust implementation to avoid depending on openssl on macOS and Win
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
mod rustls_tls {
    use std::sync::Arc;

    use tokio_rustls::{
        rustls::{self, Certificate, ClientConfig, ServerCertVerified, ServerCertVerifier},
        webpki::DNSNameRef,
    };

    use crate::{Error, Result};

    /// Create `rustls::ClientConfig`.
    pub fn rustls_client_config(
        identity_pem: Option<&Vec<u8>>,
        root_cert: Option<&Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<ClientConfig> {
        use rustls::internal::pemfile;
        use std::io::Cursor;

        // Based on code from `reqwest`
        let mut client_config = ClientConfig::new();
        if let Some(buf) = identity_pem {
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
                    .add(&Certificate(der.to_owned()))
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
