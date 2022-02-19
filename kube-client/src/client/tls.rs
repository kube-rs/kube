#[cfg(feature = "native-tls")]
pub mod native_tls {
    use thiserror::Error;
    use tokio_native_tls::native_tls::{Certificate, Identity, TlsConnector};

    const IDENTITY_PASSWORD: &str = " ";

    /// Errors from native TLS
    #[derive(Debug, Error)]
    pub enum Error {
        /// Failed to deserialize PEM-encoded X509 certificate
        #[error("failed to deserialize PEM-encoded X509 certificate: {0}")]
        DeserializeCertificate(#[source] openssl::error::ErrorStack),

        /// Failed to deserialize PEM-encoded private key
        #[error("failed to deserialize PEM-encoded private key: {0}")]
        DeserializePrivateKey(#[source] openssl::error::ErrorStack),

        /// Failed to create PKCS #12 archive
        #[error("failed to create PKCS #12 archive: {0}")]
        CreatePkcs12(#[source] openssl::error::ErrorStack),

        /// Failed to serialize PKCS #12 archive to DER
        #[error("failed to serialize PKCS #12 archive to DER encoding: {0}")]
        SerializePkcs12(#[source] openssl::error::ErrorStack),

        /// Failed to deserialize DER-encoded PKCS #12 archive
        #[error("failed to deserialize DER-encoded PKCS #12 archive: {0}")]
        DeserializePkcs12(#[source] tokio_native_tls::native_tls::Error),

        /// Failed to deserialize DER-encoded X509 certificate
        #[error("failed to deserialize DER-encoded X509 certificate: {0}")]
        DeserializeRootCertificate(#[source] tokio_native_tls::native_tls::Error),

        /// Failed to create `TlsConnector`
        #[error("failed to create `TlsConnector`: {0}")]
        CreateTlsConnector(#[source] tokio_native_tls::native_tls::Error),
    }

    /// Create `native_tls::TlsConnector`.
    pub fn native_tls_connector(
        identity_pem: Option<&Vec<u8>>,
        root_cert: Option<&Vec<Vec<u8>>>,
        accept_invalid: bool,
    ) -> Result<TlsConnector, Error> {
        let mut builder = TlsConnector::builder();
        if let Some(pem) = identity_pem {
            let identity = pkcs12_from_pem(pem, IDENTITY_PASSWORD)?;
            builder.identity(
                Identity::from_pkcs12(&identity, IDENTITY_PASSWORD).map_err(Error::DeserializePkcs12)?,
            );
        }

        if let Some(ders) = root_cert {
            for der in ders {
                builder.add_root_certificate(
                    Certificate::from_der(der).map_err(Error::DeserializeRootCertificate)?,
                );
            }
        }

        if accept_invalid {
            builder.danger_accept_invalid_certs(true);
        }

        builder.build().map_err(Error::CreateTlsConnector)
    }

    // TODO Switch to PKCS8 support when https://github.com/sfackler/rust-native-tls/pull/209 is merged
    fn pkcs12_from_pem(pem: &[u8], password: &str) -> Result<Vec<u8>, Error> {
        use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
        let x509 = X509::from_pem(pem).map_err(Error::DeserializeCertificate)?;
        let pkey = PKey::private_key_from_pem(pem).map_err(Error::DeserializePrivateKey)?;
        let p12 = Pkcs12::builder()
            .build(password, "kubeconfig", &pkey, &x509)
            .map_err(Error::CreatePkcs12)?;
        p12.to_der().map_err(Error::SerializePkcs12)
    }
}

#[cfg(feature = "rustls-tls")]
pub mod rustls_tls {
    use hyper_rustls::ConfigBuilderExt;
    use rustls::{
        self,
        client::{ServerCertVerified, ServerCertVerifier},
        Certificate, ClientConfig, PrivateKey,
    };
    use thiserror::Error;

    /// Errors from Rustls
    #[derive(Debug, Error)]
    pub enum Error {
        /// Identity PEM is invalid
        #[error("identity PEM is invalid: {0}")]
        InvalidIdentityPem(#[source] std::io::Error),

        /// Identity PEM is missing a private key: the key must be PKCS8 or RSA/PKCS1
        #[error("identity PEM is missing a private key: the key must be PKCS8 or RSA/PKCS1")]
        MissingPrivateKey,

        /// Identity PEM is missing certificate
        #[error("identity PEM is missing certificate")]
        MissingCertificate,

        /// Invalid private key
        #[error("invalid private key: {0}")]
        InvalidPrivateKey(#[source] rustls::Error),

        /// Unknown private key format
        #[error("unknown private key format")]
        UnknownPrivateKeyFormat,

        // Using type-erased error to avoid depending on webpki
        /// Failed to add a root certificate
        #[error("failed to add a root certificate: {0}")]
        AddRootCertificate(#[source] Box<dyn std::error::Error + Send + Sync>),
    }

    /// Create `rustls::ClientConfig`.
    pub fn rustls_client_config(
        identity_pem: Option<&[u8]>,
        root_certs: Option<&[Vec<u8>]>,
        accept_invalid: bool,
    ) -> Result<ClientConfig, Error> {
        let config_builder = if let Some(certs) = root_certs {
            ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_store(certs)?)
        } else {
            ClientConfig::builder().with_safe_defaults().with_native_roots()
        };

        let mut client_config = if let Some((chain, pkey)) = identity_pem.map(client_auth).transpose()? {
            config_builder
                .with_single_cert(chain, pkey)
                .map_err(Error::InvalidPrivateKey)?
        } else {
            config_builder.with_no_client_auth()
        };

        if accept_invalid {
            client_config
                .dangerous()
                .set_certificate_verifier(std::sync::Arc::new(NoCertificateVerification {}));
        }
        Ok(client_config)
    }

    fn root_store(root_certs: &[Vec<u8>]) -> Result<rustls::RootCertStore, Error> {
        let mut root_store = rustls::RootCertStore::empty();
        for der in root_certs {
            root_store
                .add(&Certificate(der.clone()))
                .map_err(|e| Error::AddRootCertificate(Box::new(e)))?;
        }
        Ok(root_store)
    }

    fn client_auth(data: &[u8]) -> Result<(Vec<Certificate>, PrivateKey), Error> {
        use rustls_pemfile::Item;

        let mut cert_chain = Vec::new();
        let mut pkcs8_key = None;
        let mut rsa_key = None;
        let mut ec_key = None;
        let mut reader = std::io::Cursor::new(data);
        for item in rustls_pemfile::read_all(&mut reader).map_err(Error::InvalidIdentityPem)? {
            match item {
                Item::X509Certificate(cert) => cert_chain.push(Certificate(cert)),
                Item::PKCS8Key(key) => pkcs8_key = Some(PrivateKey(key)),
                Item::RSAKey(key) => rsa_key = Some(PrivateKey(key)),
                Item::ECKey(key) => ec_key = Some(PrivateKey(key)),
                _ => return Err(Error::UnknownPrivateKeyFormat),
            }
        }

        let private_key = pkcs8_key.or(rsa_key).or(ec_key).ok_or(Error::MissingPrivateKey)?;
        if cert_chain.is_empty() {
            return Err(Error::MissingCertificate);
        }
        Ok((cert_chain, private_key))
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
