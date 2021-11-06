#[cfg(feature = "native-tls")]
pub mod native_tls {
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
                    Certificate::from_der(der).map_err(|e| Error::SslError(format!("{}", e)))?,
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
        // TODO These are all treated as the same error. Add specific errors.
        let x509 = X509::from_pem(pem).map_err(Error::OpensslError)?;
        let pkey = PKey::private_key_from_pem(pem).map_err(Error::OpensslError)?;
        let p12 = Pkcs12::builder()
            .build(password, "kubeconfig", &pkey, &x509)
            .map_err(Error::OpensslError)?;
        let der = p12.to_der().map_err(Error::OpensslError)?;
        Ok(der)
    }
}

#[cfg(feature = "rustls-tls")]
pub mod rustls_tls {
    use rustls::{
        self,
        client::{ServerCertVerified, ServerCertVerifier},
        Certificate, ClientConfig,
    };

    use crate::{Error, Result};

    /// Create `rustls::ClientConfig`.
    pub fn rustls_client_config(
        identity_pem: Option<&Vec<u8>>,
        root_certs: Option<&Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<ClientConfig> {
        use std::io::Cursor;

        // Create a `rustls::RootCertStore`
        let mut roots = rustls::RootCertStore::empty();
        if let Some(ders) = root_certs {
            for der in ders {
                // NB: might have to use RootCertStore::add_parsable_certificates instead
                roots
                    .add(&Certificate(der.to_owned()))
                    .map_err(|e| Error::SslError(format!("{}", e)))?;
            }
        }

        // rustls client config require a complicated series of steps through an ordered builder
        // See https://docs.rs/rustls/0.20.0/rustls/struct.ConfigBuilder.html

        let cfgbld = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(roots);

        // Convert the certs into a single cert_chain for rustls
        let client_config = if let Some(buf) = identity_pem {
            let (key, certs) = {
                let mut pem = Cursor::new(buf);
                let certs = rustls_pemfile::certs(&mut pem)
                    .and_then(|certs| {
                        if certs.is_empty() {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "No X.509 Certificates Found",
                            ))
                        } else {
                            Ok(certs.into_iter().map(rustls::Certificate).collect::<Vec<_>>())
                        }
                    })
                    .map_err(|_| Error::SslError("No valid certificate was found".into()))?;
                pem.set_position(0);

                // TODO Support EC Private Key to support k3d. Need to convert to PKCS#8 or RSA (PKCS#1).
                // `openssl pkcs8 -topk8 -nocrypt -in ec.pem -out pkcs8.pem`
                // https://wiki.openssl.org/index.php/Command_Line_Elliptic_Curve_Operations#EC_Private_Key_File_Formats
                let mut sk = rustls_pemfile::pkcs8_private_keys(&mut pem)
                    .and_then(|keys| {
                        if keys.is_empty() {
                            Err(std::io::Error::new(
                                std::io::ErrorKind::NotFound,
                                "No PKCS8 Key Found",
                            ))
                        } else {
                            Ok(keys.into_iter().map(rustls::PrivateKey).collect::<Vec<_>>())
                        }
                    })
                    .or_else(|_| {
                        pem.set_position(0);
                        rustls_pemfile::rsa_private_keys(&mut pem).and_then(|keys| {
                            if keys.is_empty() {
                                Err(std::io::Error::new(
                                    std::io::ErrorKind::NotFound,
                                    "No RSA Key Found",
                                ))
                            } else {
                                Ok(keys.into_iter().map(rustls::PrivateKey).collect::<Vec<_>>())
                            }
                        })
                    })
                    .map_err(|_| Error::SslError("No valid private key was found".into()))?;

                if let (Some(sk), false) = (sk.pop(), certs.is_empty()) {
                    (sk, certs)
                } else {
                    return Err(Error::SslError("private key or certificate not found".into()));
                }
            };
            cfgbld
                .with_single_cert(certs, key)
                .map_err(|e| Error::SslError(format!("{}", e)))?
        } else if accept_invalid || true {
            let mut cfgbld = cfgbld.with_no_client_auth();
            cfgbld
                .dangerous()
                .set_certificate_verifier(std::sync::Arc::new(NoCertificateVerification {}));
            cfgbld
        } else {
            cfgbld.with_no_client_auth()
        };

        Ok(client_config)
    }

    struct NoCertificateVerification {}

    impl ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &Certificate,
            _intermediates: &[Certificate],
            _server_name: &rustls::client::ServerName,
            _scts: &mut dyn Iterator<Item = &[u8]>,
            _ocsp_response: &[u8],
            _now: std::time::SystemTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }
    }
}
