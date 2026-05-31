#[cfg(feature = "rustls-tls")]
pub mod rustls_tls {
    use std::{
        path::PathBuf,
        sync::{Arc, RwLock},
        time::{Duration, Instant},
    };

    use hyper_rustls::ConfigBuilderExt;
    use rustls::{
        self, ClientConfig, DigitallySignedStruct, RootCertStore,
        client::{
            WebPkiServerVerifier,
            danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
        },
        pki_types::{CertificateDer, InvalidDnsNameError, PrivateKeyDer, ServerName},
    };
    use thiserror::Error;

    /// Errors from Rustls
    #[derive(Debug, Error)]
    pub enum Error {
        /// Identity PEM is invalid
        #[error("identity PEM is invalid: {0}")]
        InvalidIdentityPem(#[source] rustls::pki_types::pem::Error),

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

        /// No valid native root CA certificates found
        #[error("no valid native root CA certificates found")]
        NoValidNativeRootCA(#[source] std::io::Error),

        /// Invalid server name
        #[error("invalid server name: {0}")]
        InvalidServerName(#[source] InvalidDnsNameError),
    }

    /// Create `rustls::ClientConfig`.
    pub fn rustls_client_config(
        identity_pem: Option<&[u8]>,
        root_certs: Option<&[Vec<u8>]>,
        accept_invalid: bool,
    ) -> Result<ClientConfig, Error> {
        let config_builder = if let Some(certs) = root_certs {
            ClientConfig::builder().with_root_certificates(root_store(certs)?)
        } else {
            #[cfg(feature = "webpki-roots")]
            {
                // Use WebPKI roots.
                ClientConfig::builder().with_webpki_roots()
            }
            #[cfg(not(feature = "webpki-roots"))]
            {
                // Use native roots. This will panic on Android and iOS.
                ClientConfig::builder()
                    .with_native_roots()
                    .map_err(Error::NoValidNativeRootCA)?
            }
        };

        let mut client_config = if let Some((chain, pkey)) = identity_pem.map(client_auth).transpose()? {
            config_builder
                .with_client_auth_cert(chain, pkey)
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
                .add(CertificateDer::from(der.to_owned()))
                .map_err(|e| Error::AddRootCertificate(Box::new(e)))?;
        }
        Ok(root_store)
    }

    /// A [`ServerCertVerifier`] that re-reads the CA bundle file roughly once
    /// per minute to pick up CA rotation.
    ///
    /// This mirrors the token-file reload behaviour in
    /// [`crate::client::auth::TokenFile`]: the service-account `ca.crt` lives
    /// in the same projected volume and rotates under the same mechanism.
    /// Existing TLS connections are unaffected (they already handshook); new
    /// connections use the freshly loaded roots.
    ///
    /// If a reload fails (file missing, parse error), the last successfully
    /// loaded verifier is retained — same policy as `TokenFile`, per
    /// <https://github.com/kubernetes/kubernetes/issues/68164>.
    #[derive(Debug)]
    pub(crate) struct ReloadingVerifier {
        path: PathBuf,
        inner: RwLock<(Arc<WebPkiServerVerifier>, Instant)>,
    }

    impl ReloadingVerifier {
        const RELOAD_INTERVAL: Duration = Duration::from_secs(60);

        pub(crate) fn new(path: PathBuf) -> Result<Self, Error> {
            let verifier = Self::load(&path)?;
            Ok(Self {
                path,
                inner: RwLock::new((verifier, Instant::now())),
            })
        }

        fn load(path: &PathBuf) -> Result<Arc<WebPkiServerVerifier>, Error> {
            let pem = std::fs::read(path).map_err(|e| Error::AddRootCertificate(Box::new(e)))?;
            let ders = crate::config::certs(&pem).map_err(|e| Error::AddRootCertificate(Box::new(e)))?;
            let mut store = RootCertStore::empty();
            for der in ders {
                store
                    .add(CertificateDer::from(der))
                    .map_err(|e| Error::AddRootCertificate(Box::new(e)))?;
            }
            WebPkiServerVerifier::builder(Arc::new(store))
                .build()
                .map_err(|e| Error::AddRootCertificate(Box::new(e)))
        }

        fn current(&self) -> Arc<WebPkiServerVerifier> {
            {
                let guard = self.inner.read().unwrap_or_else(|e| e.into_inner());
                if guard.1.elapsed() < Self::RELOAD_INTERVAL {
                    return guard.0.clone();
                }
            }
            let mut guard = self.inner.write().unwrap_or_else(|e| e.into_inner());
            if guard.1.elapsed() < Self::RELOAD_INTERVAL {
                return guard.0.clone();
            }
            if let Ok(fresh) = Self::load(&self.path) {
                guard.0 = fresh;
            } else {
                tracing::warn!(path = ?self.path, "failed to reload CA bundle; keeping stale roots");
            }
            guard.1 = Instant::now();
            guard.0.clone()
        }
    }

    impl ServerCertVerifier for ReloadingVerifier {
        fn verify_server_cert(
            &self,
            end_entity: &CertificateDer,
            intermediates: &[CertificateDer],
            server_name: &ServerName,
            ocsp_response: &[u8],
            now: rustls::pki_types::UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            self.current()
                .verify_server_cert(end_entity, intermediates, server_name, ocsp_response, now)
        }

        fn verify_tls12_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            self.current().verify_tls12_signature(message, cert, dss)
        }

        fn verify_tls13_signature(
            &self,
            message: &[u8],
            cert: &CertificateDer,
            dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            self.current().verify_tls13_signature(message, cert, dss)
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            self.current().supported_verify_schemes()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // EC P-256 self-signed CAs, valid until 2126. Regenerate with:
        //   openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
        //     -keyout /dev/null -out ca.pem -days 36500 -subj "/CN=test-ca-N"
        const CA1: &str = "-----BEGIN CERTIFICATE-----
MIIBgDCCASWgAwIBAgIUVrQf5d//S01a0fbXxYRIx9wc0VQwCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJdGVzdC1jYS0xMCAXDTI2MDMwNDEzMDk1MFoYDzIxMjYwMjA4
MTMwOTUwWjAUMRIwEAYDVQQDDAl0ZXN0LWNhLTEwWTATBgcqhkjOPQIBBggqhkjO
PQMBBwNCAARg57mWJPDsAIEQAgXqMOOfjMQP+PE9HqcZobycO8z94r/uRuV0wKx/
0SvMsKFtnreut0bjgFtmZaWY+6d87Is9o1MwUTAdBgNVHQ4EFgQUjtGuhkM7LtHB
gMPCJIxMwbY69OQwHwYDVR0jBBgwFoAUjtGuhkM7LtHBgMPCJIxMwbY69OQwDwYD
VR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNJADBGAiEAj/WzNVJDg/cBtLqQVM77
tkB+QyIXLG3Vi9Xj1YfW9QECIQDDFW8yFtgLeCg2Zhr4xQNq3/24r/01kI2rjFPO
xBkDMw==
-----END CERTIFICATE-----
";
        const CA2: &str = "-----BEGIN CERTIFICATE-----
MIIBfjCCASWgAwIBAgIUZ7Qsiwan2joRz01p25/cy1XNNiwwCgYIKoZIzj0EAwIw
FDESMBAGA1UEAwwJdGVzdC1jYS0yMCAXDTI2MDMwNDEzMDk1MFoYDzIxMjYwMjA4
MTMwOTUwWjAUMRIwEAYDVQQDDAl0ZXN0LWNhLTIwWTATBgcqhkjOPQIBBggqhkjO
PQMBBwNCAARJle2/yiOD5zp0UkjZg9Yy6ZHBItTLrqv/uzB2YMQg03frnqEUMzSV
mFinosBcGpX/dPGfHNPhBMOpHmlocZu9o1MwUTAdBgNVHQ4EFgQUsqG0hSGDYsz2
eGIsLIwJnCR5SFIwHwYDVR0jBBgwFoAUsqG0hSGDYsz2eGIsLIwJnCR5SFIwDwYD
VR0TAQH/BAUwAwEB/zAKBggqhkjOPQQDAgNHADBEAiApvLu9DIC3/K/+G9ooOm75
a72Cjw62aM8NfPe7ILs8SgIgL0VHe6ksTyB176RECCm3MJVnlhOop6b1tNvxjrru
FRU=
-----END CERTIFICATE-----
";

        fn expire(v: &ReloadingVerifier) {
            // Can't move Instant backwards; instead, reach past the guard.
            // The test pokes private state the same way auth::tests::token_file does.
            v.inner.write().unwrap().1 = Instant::now().checked_sub(Duration::from_secs(120)).unwrap();
        }

        #[test]
        fn reloading_verifier() {
            #[cfg(feature = "aws-lc-rs")]
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let file = tempfile::NamedTempFile::new().unwrap();
            std::fs::write(file.path(), CA1).unwrap();

            let verifier = ReloadingVerifier::new(file.path().to_path_buf()).unwrap();
            let first = verifier.current();

            // File changed but we're still within the reload interval: no reload.
            std::fs::write(file.path(), CA2).unwrap();
            assert!(Arc::ptr_eq(&verifier.current(), &first));

            // Force expiry: reload picks up CA2.
            expire(&verifier);
            let second = verifier.current();
            assert!(!Arc::ptr_eq(&second, &first));

            // File gone, expired again: keep stale verifier.
            drop(file);
            expire(&verifier);
            assert!(Arc::ptr_eq(&verifier.current(), &second));
        }
    }

    fn client_auth(data: &[u8]) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), Error> {
        use rustls::pki_types::pem::{self, SectionKind};

        let mut cert_chain = Vec::new();
        let mut pkcs8_key = None;
        let mut pkcs1_key = None;
        let mut sec1_key = None;
        let mut reader = std::io::Cursor::new(data);
        while let Some((kind, der)) = pem::from_buf(&mut reader).map_err(Error::InvalidIdentityPem)? {
            match kind {
                SectionKind::Certificate => cert_chain.push(der.into()),
                SectionKind::PrivateKey => pkcs8_key = Some(PrivateKeyDer::Pkcs8(der.into())),
                SectionKind::RsaPrivateKey => pkcs1_key = Some(PrivateKeyDer::Pkcs1(der.into())),
                SectionKind::EcPrivateKey => sec1_key = Some(PrivateKeyDer::Sec1(der.into())),
                _ => return Err(Error::UnknownPrivateKeyFormat),
            }
        }

        let private_key = pkcs8_key
            .or(pkcs1_key)
            .or(sec1_key)
            .ok_or(Error::MissingPrivateKey)?;
        if cert_chain.is_empty() {
            return Err(Error::MissingCertificate);
        }
        Ok((cert_chain, private_key))
    }

    #[derive(Debug)]
    struct NoCertificateVerification {}

    impl ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer,
            _intermediates: &[CertificateDer],
            _server_name: &ServerName,
            _ocsp_response: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            tracing::warn!("Server cert bypassed");
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            use rustls::SignatureScheme;
            vec![
                SignatureScheme::RSA_PKCS1_SHA1,
                SignatureScheme::ECDSA_SHA1_Legacy,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::ECDSA_NISTP521_SHA512,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::ED25519,
                SignatureScheme::ED448,
            ]
        }
    }

    /// HTTP/1.1-only HTTPS connector that mirrors `hyper_rustls::HttpsConnector`
    /// but allows specifying both an explicit ALPN advertisement and a custom
    /// TLS server name.
    ///
    /// Why this exists: hyper-rustls' builder asserts that the rustls
    /// `ClientConfig`'s `alpn_protocols` is empty in `with_tls_config`, and
    /// only `enable_http2()` populates it afterwards. The `enable_http1()`-only
    /// builder path therefore leaves ALPN empty -- which means no ALPN
    /// extension is sent on the wire and a modern apiserver may still
    /// negotiate HTTP/2. We need an explicit `http/1.1` advertisement to force
    /// HTTP/1.1 for the upgrade transport, otherwise upgrades break.
    ///
    /// The `From<(H, Arc<ClientConfig>)>` impl on `hyper_rustls::HttpsConnector`
    /// would let us bypass the assertion, but it constructs the connector
    /// with the *default* server-name resolver and there is no public API to
    /// swap that resolver afterwards. Users who set `Config::tls_server_name`
    /// would silently lose SNI override on the upgrade transport, breaking
    /// cert validation against alternate-host clusters. So we reimplement
    /// the small TCP-then-TLS dance here over public hyper-rustls types,
    /// which lets us honour `tls_server_name` without an upstream change.
    pub struct H1OnlyHttpsConnector<H> {
        http: H,
        tls_config: std::sync::Arc<ClientConfig>,
        server_name: Option<ServerName<'static>>,
    }

    impl<H: Clone> Clone for H1OnlyHttpsConnector<H> {
        fn clone(&self) -> Self {
            Self {
                http: self.http.clone(),
                tls_config: self.tls_config.clone(),
                server_name: self.server_name.clone(),
            }
        }
    }

    impl<H> H1OnlyHttpsConnector<H> {
        pub fn new(http: H, mut tls_config: ClientConfig, server_name: Option<ServerName<'static>>) -> Self {
            tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
            Self {
                http,
                tls_config: std::sync::Arc::new(tls_config),
                server_name,
            }
        }
    }

    impl<H> tower::Service<http::Uri> for H1OnlyHttpsConnector<H>
    where
        H: tower::Service<http::Uri> + Send + Clone + 'static,
        H::Response: hyper::rt::Read
            + hyper::rt::Write
            + hyper_util::client::legacy::connect::Connection
            + Unpin
            + Send
            + 'static,
        H::Future: Send + 'static,
        H::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        type Error = std::io::Error;
        type Future = std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Self::Response, std::io::Error>> + Send>,
        >;
        type Response = hyper_rustls::MaybeHttpsStream<H::Response>;

        fn poll_ready(
            &mut self,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            self.http
                .poll_ready(cx)
                .map_err(|e| std::io::Error::other(e.into()))
        }

        fn call(&mut self, dst: http::Uri) -> Self::Future {
            // Fall back to plain HTTP for `http://` URIs, matching
            // hyper_rustls' default behaviour.
            if dst.scheme() == Some(&http::uri::Scheme::HTTP) {
                let fut = self.http.call(dst);
                return Box::pin(async move {
                    let s = fut.await.map_err(|e| std::io::Error::other(e.into()))?;
                    Ok(hyper_rustls::MaybeHttpsStream::Http(s))
                });
            }
            if dst.scheme() != Some(&http::uri::Scheme::HTTPS) {
                let scheme = dst.scheme().map(|s| s.to_string()).unwrap_or_default();
                return Box::pin(async move {
                    Err(std::io::Error::other(format!("unsupported scheme {scheme}")))
                });
            }

            // Resolve SNI: explicit override wins, else use the URI host.
            let sni = match self.server_name.clone() {
                Some(name) => Ok(name),
                None => match dst.host() {
                    Some(host) => {
                        // Strip surrounding brackets on IPv6 literals.
                        let host = host.trim_start_matches('[').trim_end_matches(']');
                        ServerName::try_from(host.to_owned()).map_err(Error::InvalidServerName)
                    }
                    None => {
                        return Box::pin(async move { Err(std::io::Error::other("missing host in URI")) });
                    }
                },
            };
            let sni = match sni {
                Ok(s) => s,
                Err(e) => {
                    return Box::pin(async move { Err(std::io::Error::other(e)) });
                }
            };

            let cfg = self.tls_config.clone();
            let connecting = self.http.call(dst);
            Box::pin(async move {
                let tcp = connecting.await.map_err(|e| std::io::Error::other(e.into()))?;
                let tls = tokio_rustls::TlsConnector::from(cfg)
                    .connect(sni, hyper_util::rt::TokioIo::new(tcp))
                    .await
                    .map_err(std::io::Error::other)?;
                Ok(hyper_rustls::MaybeHttpsStream::Https(
                    hyper_util::rt::TokioIo::new(tls),
                ))
            })
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
