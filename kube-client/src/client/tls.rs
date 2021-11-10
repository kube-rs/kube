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
    use std::sync::Arc;

    use rustls::{self, Certificate, ClientConfig, ServerCertVerified, ServerCertVerifier};
    use webpki::DNSNameRef;

    use crate::{Error, Result};

    /// Create `rustls::ClientConfig`.
    pub fn rustls_client_config(
        identity_pem: Option<&Vec<u8>>,
        root_cert: Option<&Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<ClientConfig> {
        use std::io::Cursor;

        // Based on code from `reqwest`
        let mut client_config = ClientConfig::new();
        if let Some(buf) = identity_pem {
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

#[cfg(feature = "openssl-tls")]
pub mod openssl_tls {
    use openssl::{
        pkey::PKey,
        ssl::{SslConnector, SslConnectorBuilder, SslMethod},
        x509::X509,
    };
    use thiserror::Error;

    /// Errors from OpenSSL TLS
    #[derive(Debug, Error)]
    pub enum Error {
        /// Failed to create OpenSSL HTTPS connector
        #[error("failed to create OpenSSL HTTPS connector: {0}")]
        CreateHttpsConnector(#[source] openssl::error::ErrorStack),

        /// Failed to create OpenSSL SSL connector
        #[error("failed to create OpenSSL SSL connector: {0}")]
        CreateSslConnector(#[source] SslConnectorError),
    }

    /// Errors from creating a `SslConnectorBuilder`
    #[derive(Debug, Error)]
    pub enum SslConnectorError {
        /// Failed to build SslConnectorBuilder
        #[error("failed to build SslConnectorBuilder: {0}")]
        CreateBuilder(#[source] openssl::error::ErrorStack),

        /// Failed to deserialize PEM-encoded chain of certificates
        #[error("failed to deserialize PEM-encoded chain of certificates: {0}")]
        DeserializeCertificateChain(#[source] openssl::error::ErrorStack),

        /// Failed to deserialize PEM-encoded private key
        #[error("failed to deserialize PEM-encoded private key: {0}")]
        DeserializePrivateKey(#[source] openssl::error::ErrorStack),

        /// Failed to set private key
        #[error("failed to set private key: {0}")]
        SetPrivateKey(#[source] openssl::error::ErrorStack),

        /// Failed to get a leaf certificate, the certificate chain is empty
        #[error("failed to get a leaf certificate, the certificate chain is empty")]
        GetLeafCertificate,

        /// Failed to set the leaf certificate
        #[error("failed to set the leaf certificate: {0}")]
        SetLeafCertificate(#[source] openssl::error::ErrorStack),

        /// Failed to append a certificate to the chain
        #[error("failed to append a certificate to the chain: {0}")]
        AppendCertificate(#[source] openssl::error::ErrorStack),

        /// Failed to deserialize DER-encoded root certificate
        #[error("failed to deserialize DER-encoded root certificate: {0}")]
        DeserializeRootCertificate(#[source] openssl::error::ErrorStack),

        /// Failed to add a root certificate
        #[error("failed to add a root certificate: {0}")]
        AddRootCertificate(#[source] openssl::error::ErrorStack),
    }

    /// Create `openssl::ssl::SslConnectorBuilder` required for `hyper_openssl::HttpsConnector`.
    pub fn ssl_connector_builder(
        identity_pem: Option<&Vec<u8>>,
        root_certs: Option<&Vec<Vec<u8>>>,
    ) -> Result<SslConnectorBuilder, SslConnectorError> {
        let mut builder =
            SslConnector::builder(SslMethod::tls()).map_err(SslConnectorError::CreateBuilder)?;
        if let Some(pem) = identity_pem {
            let mut chain = X509::stack_from_pem(pem)
                .map_err(SslConnectorError::DeserializeCertificateChain)?
                .into_iter();
            let leaf_cert = chain.next().ok_or(SslConnectorError::GetLeafCertificate)?;
            builder
                .set_certificate(&leaf_cert)
                .map_err(SslConnectorError::SetLeafCertificate)?;
            for cert in chain {
                builder
                    .add_extra_chain_cert(cert)
                    .map_err(SslConnectorError::AppendCertificate)?;
            }

            let pkey = PKey::private_key_from_pem(pem).map_err(SslConnectorError::DeserializePrivateKey)?;
            builder
                .set_private_key(&pkey)
                .map_err(SslConnectorError::SetPrivateKey)?;
        }

        if let Some(ders) = root_certs {
            for der in ders {
                let cert = X509::from_der(der).map_err(SslConnectorError::DeserializeRootCertificate)?;
                builder
                    .cert_store_mut()
                    .add_cert(cert)
                    .map_err(SslConnectorError::AddRootCertificate)?;
            }
        }

        Ok(builder)
    }
}
