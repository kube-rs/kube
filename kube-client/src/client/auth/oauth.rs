use tame_oauth::{
    gcp::{TokenOrRequest, TokenProvider, TokenProviderWrapper},
    Token,
};
use thiserror::Error;

#[derive(Error, Debug)]
/// Possible errors when requesting token with OAuth
pub enum Error {
    /// Default provider appears to be configured, but was invalid
    #[error("default provider is configured but invalid: {0}")]
    InvalidDefaultProviderConfig(#[source] tame_oauth::Error),

    /// No provider was found
    #[error("no provider was found")]
    NoDefaultProvider,

    /// Failed to load OAuth credentials file
    #[error("failed to load OAuth credentials file: {0}")]
    LoadCredentials(#[source] std::io::Error),

    /// Failed to parse OAuth credentials file
    #[error("failed to parse OAuth credentials file: {0}")]
    ParseCredentials(#[source] serde_json::Error),

    /// Credentials file had invalid key format
    #[error("credentials file had invalid key format: {0}")]
    InvalidKeyFormat(#[source] tame_oauth::Error),

    /// Credentials file had invalid RSA key
    #[error("credentials file had invalid RSA key: {0}")]
    InvalidRsaKey(#[source] tame_oauth::Error),

    /// Failed to request token
    #[error("failed to request token: {0}")]
    RequestToken(#[source] hyper::Error),

    /// Failed to retrieve new credential
    #[error("failed to retrieve new credential {0:?}")]
    RetrieveCredentials(#[source] tame_oauth::Error),

    /// Failed to parse token
    #[error("failed to parse token: {0}")]
    ParseToken(#[source] serde_json::Error),

    /// Failed to concatenate the buffers from response body
    #[error("failed to concatenate the buffers from response body: {0}")]
    ConcatBuffers(#[source] hyper::Error),

    /// Failed to build a request
    #[error("failed to build request: {0}")]
    BuildRequest(#[source] http::Error),

    /// OAuth failed with unknown reason
    #[error("unknown OAuth error: {0}")]
    Unknown(String),

    /// Failed to create OpenSSL HTTPS connector
    #[cfg(feature = "openssl-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "openssl-tls")))]
    #[error("failed to create OpenSSL HTTPS connector: {0}")]
    CreateOpensslHttpsConnector(#[source] openssl::error::ErrorStack),
}

pub struct Gcp {
    provider: TokenProviderWrapper,
    scopes: Vec<String>,
}

impl std::fmt::Debug for Gcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gcp")
            .field("provider", &"{}".to_owned())
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl Gcp {
    // Initialize `TokenProvider` following the "Google Default Credentials" flow.
    // `tame-oauth` supports the same default credentials flow as the Go oauth2:
    // - `GOOGLE_APPLICATION_CREDENTIALS` environmment variable
    // - gcloud's application default credentials
    // - local metadata server if running on GCP
    pub(crate) fn default_credentials_with_scopes(scopes: Option<&String>) -> Result<Self, Error> {
        const DEFAULT_SCOPES: &str =
            "https://www.googleapis.com/auth/cloud-platform,https://www.googleapis.com/auth/userinfo.email";

        let provider = TokenProviderWrapper::get_default_provider()
            .map_err(Error::InvalidDefaultProviderConfig)?
            .ok_or(Error::NoDefaultProvider)?;
        let scopes = scopes
            .map(String::to_owned)
            .unwrap_or_else(|| DEFAULT_SCOPES.to_owned())
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(Self { provider, scopes })
    }

    pub async fn token(&self) -> Result<Token, Error> {
        match self.provider.get_token(&self.scopes) {
            Ok(TokenOrRequest::Request {
                request, scope_hash, ..
            }) => {
                #[cfg(not(any(feature = "rustls-tls", feature = "openssl-tls")))]
                compile_error!(
                    "At least one of rustls-tls or openssl-tls feature must be enabled to use oauth feature"
                );
                // Current TLS feature precedence when more than one are set:
                // 1. openssl-tls
                // 2. rustls-tls
                #[cfg(feature = "openssl-tls")]
                let https =
                    hyper_openssl::HttpsConnector::new().map_err(Error::CreateOpensslHttpsConnector)?;
                #[cfg(all(not(feature = "openssl-tls"), feature = "rustls-tls"))]
                let https = hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .https_only()
                    .enable_http1()
                    .build();

                let client = hyper::Client::builder().build::<_, hyper::Body>(https);

                let res = client
                    .request(request.map(hyper::Body::from))
                    .await
                    .map_err(Error::RequestToken)?;
                // Convert response body to `Vec<u8>` for parsing.
                let (parts, body) = res.into_parts();
                let bytes = hyper::body::to_bytes(body).await.map_err(Error::ConcatBuffers)?;
                let response = http::Response::from_parts(parts, bytes.to_vec());
                match self.provider.parse_token_response(scope_hash, response) {
                    Ok(token) => Ok(token),

                    Err(err) => Err(match err {
                        tame_oauth::Error::Auth(_) | tame_oauth::Error::HttpStatus(_) => {
                            Error::RetrieveCredentials(err)
                        }
                        tame_oauth::Error::Json(e) => Error::ParseToken(e),
                        err => Error::Unknown(err.to_string()),
                    }),
                }
            }

            Ok(TokenOrRequest::Token(token)) => Ok(token),

            Err(err) => match err {
                tame_oauth::Error::Http(e) => Err(Error::BuildRequest(e)),
                tame_oauth::Error::InvalidRsaKey(_) => Err(Error::InvalidRsaKey(err)),
                tame_oauth::Error::InvalidKeyFormat => Err(Error::InvalidKeyFormat(err)),
                e => Err(Error::Unknown(e.to_string())),
            },
        }
    }
}
