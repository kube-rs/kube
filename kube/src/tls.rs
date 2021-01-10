use crate::config::Der;
use crate::Result;

#[cfg(feature = "native-tls")]
mod native_impl;
mod rustls_impl;
/// Abstraction layer for TLS implementations
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub enum Tls {
    /// Use `rustls`
    #[cfg(feature = "native-tls")]
    Native,
    /// Use `openssl`
    #[cfg(feature = "rustls-tls")]
    Rustls,
}

impl Tls {
    /// Creates a TLS backend.
    /// This function only works when exactly one backend
    /// was configured, otherwise it will panic. This function
    /// is only intended to use in tests.
    #[allow(unreachable_code)]
    pub fn pick() -> Tls {
        let mut backend_count = 0;
        if cfg!(feature = "native-tls") {
            backend_count += 1;
        }
        if cfg!(feature = "rustls-tls") {
            backend_count += 1;
        }
        assert_eq!(backend_count, 1);
        #[cfg(feature = "native-tls")]
        return Tls::Native;
        #[cfg(feature = "rustls-tls")]
        return Tls::Rustls;
        unreachable!()
    }

    pub(crate) fn ca_bundle(&self, bundle: &[u8]) -> Result<Vec<Der>> {
        match self {
            #[cfg(feature = "native-tls")]
            Tls::Native => native_impl::ca_bundle(bundle),
            #[cfg(feature = "rustls-tls")]
            Tls::Rustls => rustls_impl::ca_bundle(bundle),
        }
    }

    pub(crate) fn identity(&self, password: &str, client_cert: &[u8], client_key: &[u8]) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "native-tls")]
            Tls::Native => native_impl::identity(password, client_cert, client_key),
            #[cfg(feature = "rustls-tls")]
            Tls::Rustls => rustls_impl::identity(password, client_cert, client_key),
        }
    }

    pub(crate) fn sign(&self, signature_base: &str, private_key: &str) -> Result<Vec<u8>> {
        match self {
            #[cfg(feature = "native-tls")]
            Tls::Native => native_impl::sign(signature_base, private_key),
            #[cfg(feature = "rustls-tls")]
            Tls::Rustls => rustls_impl::sign(signature_base, private_key),
        }
    }

    pub(crate) fn reqwest(&self) -> reqwest::Client {
        let mut builder = reqwest::Client::builder();
        builder = match self {
            #[cfg(feature = "native-tls")]
            Tls::Native => builder.use_native_tls(),
            #[cfg(feature = "rustls-tls")]
            Tls::Rustls => builder.use_rustls_tls(),
        };
        builder.build().expect("reqwest initialization error")
    }
}
