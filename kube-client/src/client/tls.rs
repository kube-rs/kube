#[cfg(feature = "rustls-tls")]
pub mod rustls_tls {
    use std::{
        path::PathBuf,
        sync::{Arc, RwLock},
        time::Duration,
    };
    use tokio::time::Instant;

    // Only the webpki-roots fallback uses this; the default platform-verifier path does not.
    #[cfg(feature = "webpki-roots")] use hyper_rustls::ConfigBuilderExt;
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
        #[error("no valid native root CA certificates found: {0}")]
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
            // No CA was configured (e.g. the kubeconfig has no `certificate-authority`),
            // so defer to the trust store, mirroring how client-go leaves `RootCAs`
            // nil to use the OS roots.
            #[cfg(feature = "webpki-roots")]
            {
                // Opt-in override: use the bundled WebPKI (Mozilla) roots.
                ClientConfig::builder().with_webpki_roots()
            }
            #[cfg(not(feature = "webpki-roots"))]
            {
                // Default: delegate to the OS-native verifier (macOS Security
                // framework, Windows CryptoAPI, native roots on Linux/Android/iOS),
                // honouring OS trust policies such as per-certificate trust settings.
                use rustls_platform_verifier::BuilderVerifierExt;
                ClientConfig::builder()
                    .with_platform_verifier()
                    // Reuse the type-erased trust-setup error variant to keep the
                    // public `Error` enum stable.
                    .map_err(|e| Error::AddRootCertificate(Box::new(e)))?
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

    #[cfg(all(test, not(feature = "webpki-roots")))]
    mod platform_verifier_tests {
        use super::*;

        // With no kubeconfig CA, the client config must fall back to the OS-native
        // verifier instead of producing an empty (reject-everything) trust store.
        #[test]
        fn no_ca_falls_back_to_platform_verifier() {
            #[cfg(feature = "aws-lc-rs")]
            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            rustls_client_config(None, None, false).expect("platform verifier config should build");
        }
    }
}

#[cfg(feature = "openssl-tls")]
pub mod openssl_tls {
    use openssl::{
        pkey::PKey,
        ssl::{SslConnector, SslConnectorBuilder, SslMethod},
        x509::{X509, store::X509StoreBuilder},
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
            // SslConnector::builder loaded OpenSSL's default verify paths into the context's store.
            // Replace that store so only the configured roots are trusted, as in client-go and the
            // rustls path. Without configured roots, the default store stays.
            let mut store = X509StoreBuilder::new().map_err(SslConnectorError::CreateBuilder)?;
            for der in ders {
                let cert = X509::from_der(der).map_err(SslConnectorError::DeserializeRootCertificate)?;
                store
                    .add_cert(cert)
                    .map_err(SslConnectorError::AddRootCertificate)?;
            }
            builder.set_cert_store(store.build());
        }

        Ok(builder)
    }

    #[cfg(test)]
    mod tests {
        use std::{
            net::{TcpListener, TcpStream},
            path::Path,
            process::Command,
        };

        use openssl::{
            asn1::Asn1Time,
            ec::{EcGroup, EcKey},
            hash::MessageDigest,
            nid::Nid,
            pkey::Private,
            ssl::{HandshakeError, SslAcceptor, SslAcceptorBuilder, SslVerifyMode},
            x509::{
                X509Name, X509VerifyResult,
                extension::{BasicConstraints, SubjectAlternativeName},
            },
        };

        use super::*;

        const SERVER_NAME: &str = "kube.test";

        // Set in the child process to the file the "default" CA is written to. The parent points
        // SSL_CERT_FILE at the same file.
        const DEFAULT_CA_FILE_ENV: &str = "KUBE_TEST_OPENSSL_DEFAULT_CA_FILE";

        struct Issued {
            cert: X509,
            key: PKey<Private>,
        }

        // A CA cert, or a leaf with a DNS SAN for SERVER_NAME. Self-signed when `issuer` is None.
        fn issue(cn: &str, ca: bool, issuer: Option<&Issued>) -> Issued {
            let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
            let key = PKey::from_ec_key(EcKey::generate(&group).unwrap()).unwrap();

            let mut name = X509Name::builder().unwrap();
            name.append_entry_by_text("CN", cn).unwrap();
            let name = name.build();

            let mut builder = X509::builder().unwrap();
            builder.set_version(2).unwrap();
            builder.set_subject_name(&name).unwrap();
            match issuer {
                Some(issuer) => builder.set_issuer_name(issuer.cert.subject_name()).unwrap(),
                None => builder.set_issuer_name(&name).unwrap(),
            }
            builder.set_pubkey(&key).unwrap();
            builder
                .set_not_before(&Asn1Time::days_from_now(0).unwrap())
                .unwrap();
            builder
                .set_not_after(&Asn1Time::days_from_now(1).unwrap())
                .unwrap();
            if ca {
                builder
                    .append_extension(BasicConstraints::new().critical().ca().build().unwrap())
                    .unwrap();
            } else {
                let san = SubjectAlternativeName::new()
                    .dns(SERVER_NAME)
                    .build(&builder.x509v3_context(None, None))
                    .unwrap();
                builder.append_extension(san).unwrap();
            }
            let signing_key = issuer.map_or(&key, |issuer| &issuer.key);
            builder.sign(signing_key, MessageDigest::sha256()).unwrap();

            Issued {
                cert: builder.build(),
                key,
            }
        }

        fn connector(roots: Option<&[&X509]>, identity: Option<&Vec<u8>>) -> SslConnector {
            let roots = roots.map(|roots| roots.iter().map(|c| c.to_der().unwrap()).collect::<Vec<_>>());
            ssl_connector_builder(identity, roots.as_ref()).unwrap().build()
        }

        fn acceptor(leaf: &Issued, extra_chain: &[&X509]) -> SslAcceptorBuilder {
            let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls()).unwrap();
            acceptor.set_private_key(&leaf.key).unwrap();
            acceptor.set_certificate(&leaf.cert).unwrap();
            for cert in extra_chain {
                acceptor.add_extra_chain_cert((*cert).clone()).unwrap();
            }
            acceptor
        }

        // One handshake on localhost, verifying the server as `domain`. Returns the client's verify
        // error if it rejected the server certificate, and the server's handshake result.
        fn handshake(
            client: &SslConnector,
            acceptor: SslAcceptor,
            domain: &str,
        ) -> (Result<(), X509VerifyResult>, Result<(), String>) {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let addr = listener.local_addr().unwrap();
            let server = std::thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                acceptor.accept(stream).map(drop).map_err(|e| e.to_string())
            });

            match client.connect(domain, TcpStream::connect(addr).unwrap()) {
                Ok(stream) => {
                    // Keep the connection open until the server has finished its side.
                    let server = server.join().unwrap();
                    drop(stream);
                    (Ok(()), server)
                }
                Err(HandshakeError::Failure(mid)) => {
                    let verify = mid.ssl().verify_result();
                    assert_ne!(
                        verify,
                        X509VerifyResult::OK,
                        "handshake failed for a reason other than verification: {}",
                        mid.error()
                    );
                    drop(mid);
                    (Err(verify), server.join().unwrap())
                }
                Err(e) => panic!("unexpected handshake error: {e}"),
            }
        }

        // A server cert from a CA in OpenSSL's default store must be rejected once root certs are
        // configured, as client-go and the rustls path do. SSL_CERT_FILE is process-global and
        // unsafe_code is forbidden (no env::set_var), so the test re-runs itself in a child process
        // with the variable set.
        #[test]
        fn default_verify_paths_are_not_trusted_with_root_certs() {
            if let Some(file) = std::env::var_os(DEFAULT_CA_FILE_ENV) {
                return check_default_verify_paths(Path::new(&file));
            }
            // LibreSSL ignores SSL_CERT_FILE and SSL_CERT_DIR, so its default store can't be redirected.
            if openssl::version::version().starts_with("LibreSSL") {
                return;
            }

            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("default-ca.pem");
            let test = concat!(
                module_path!(),
                "::default_verify_paths_are_not_trusted_with_root_certs"
            );
            // libtest names omit the crate name.
            let test = test.split_once("::").unwrap().1;
            let output = Command::new(std::env::current_exe().unwrap())
                .args([test, "--exact", "--test-threads=1"])
                .env(DEFAULT_CA_FILE_ENV, &file)
                .env("SSL_CERT_FILE", &file)
                .env("SSL_CERT_DIR", dir.path())
                .output()
                .unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "child test failed:\n{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        fn check_default_verify_paths(default_ca_file: &Path) {
            let default_ca = issue("default-ca", true, None);
            std::fs::write(default_ca_file, default_ca.cert.to_pem().unwrap()).unwrap();
            let by_default_ca = issue("server", false, Some(&default_ca));
            let cluster_ca = issue("cluster-ca", true, None);
            let by_cluster_ca = issue("server", false, Some(&cluster_ca));

            // No root certs: the default store applies. This also shows SSL_CERT_FILE took effect.
            let (client, _) = handshake(
                &connector(None, None),
                acceptor(&by_default_ca, &[]).build(),
                SERVER_NAME,
            );
            assert!(client.is_ok(), "default store should be used without root certs");

            let pinned = connector(Some(&[&cluster_ca.cert]), None);
            let (client, _) = handshake(&pinned, acceptor(&by_cluster_ca, &[]).build(), SERVER_NAME);
            assert!(client.is_ok(), "configured root should be trusted");
            let (client, _) = handshake(&pinned, acceptor(&by_default_ca, &[]).build(), SERVER_NAME);
            assert!(
                client.is_err(),
                "default store must not be trusted when root certs are configured"
            );

            // An empty root list trusts nothing rather than falling back to the default store.
            let (client, _) = handshake(
                &connector(Some(&[]), None),
                acceptor(&by_default_ca, &[]).build(),
                SERVER_NAME,
            );
            assert!(
                client.is_err(),
                "empty root list must not fall back to the default store"
            );
        }

        #[test]
        fn root_certs_with_intermediates() {
            let root = issue("root-ca", true, None);
            let intermediate = issue("intermediate-ca", true, Some(&root));
            let leaf = issue("server", false, Some(&intermediate));

            // Bundle with root and intermediate; the server sends only its leaf.
            let bundle = connector(Some(&[&root.cert, &intermediate.cert]), None);
            let (client, _) = handshake(&bundle, acceptor(&leaf, &[]).build(), SERVER_NAME);
            assert!(
                client.is_ok(),
                "intermediate from the bundle should complete the chain"
            );

            // Bundle with only the root; the server sends the intermediate.
            let root_only = connector(Some(&[&root.cert]), None);
            let (client, _) = handshake(
                &root_only,
                acceptor(&leaf, &[&intermediate.cert]).build(),
                SERVER_NAME,
            );
            assert!(
                client.is_ok(),
                "intermediate from the server should complete the chain"
            );

            // Hostname verification is still on.
            let (client, _) = handshake(
                &root_only,
                acceptor(&leaf, &[&intermediate.cert]).build(),
                "other.test",
            );
            assert!(client.is_err(), "hostname mismatch must be rejected");
        }

        #[test]
        fn identity_chain_is_sent_with_root_certs() {
            let server_ca = issue("server-ca", true, None);
            let server = issue("server", false, Some(&server_ca));
            let client_root = issue("client-root-ca", true, None);
            let client_intermediate = issue("client-intermediate-ca", true, Some(&client_root));
            let client = issue("client", false, Some(&client_intermediate));

            let mut identity = client.cert.to_pem().unwrap();
            identity.extend(client_intermediate.cert.to_pem().unwrap());
            identity.extend(client.key.private_key_to_pem_pkcs8().unwrap());

            // The server only knows the client root, so it needs the intermediate from the client.
            let mut acceptor = acceptor(&server, &[]);
            acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);
            acceptor
                .cert_store_mut()
                .add_cert(client_root.cert.clone())
                .unwrap();

            let (client, server) = handshake(
                &connector(Some(&[&server_ca.cert]), Some(&identity)),
                acceptor.build(),
                SERVER_NAME,
            );
            assert!(client.is_ok(), "client should trust the server");
            server.expect("server should verify the client chain through the intermediate");
        }
    }
}
