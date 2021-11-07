use std::{env, path::PathBuf};

use tame_oauth::{
    gcp::{ServiceAccountAccess, ServiceAccountInfo, TokenOrRequest},
    Token,
};
use thiserror::Error;

#[derive(Error, Debug)]
/// Possible errors when requesting token with OAuth
pub enum Error {
    /// Missing `GOOGLE_APPLICATION_CREDENTIALS` env
    #[error("missing GOOGLE_APPLICATION_CREDENTIALS env")]
    MissingGoogleCredentials,

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
}

pub(crate) struct Gcp {
    access: ServiceAccountAccess,
    scopes: Vec<String>,
}

impl std::fmt::Debug for Gcp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gcp")
            .field("access", &"{}".to_owned())
            .field("scopes", &self.scopes)
            .finish()
    }
}

impl Gcp {
    pub(crate) fn new(access: ServiceAccountAccess, scopes: Vec<String>) -> Self {
        Self { access, scopes }
    }

    // Initialize ServiceAccountAccess so we can request later when needed.
    pub(crate) fn from_env_and_scopes(scopes: Option<&String>) -> Result<Self, Error> {
        const DEFAULT_SCOPES: &str =
            "https://www.googleapis.com/auth/cloud-platform,https://www.googleapis.com/auth/userinfo.email";
        // Initialize ServiceAccountAccess so we can request later when needed.
        let info = gcloud_account_info()?;
        let access = ServiceAccountAccess::new(info).map_err(Error::InvalidKeyFormat)?;
        let scopes = scopes
            .map(String::to_owned)
            .unwrap_or_else(|| DEFAULT_SCOPES.to_owned())
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(Self::new(access, scopes))
    }

    pub async fn token(&self) -> Result<Token, Error> {
        match self.access.get_token(&self.scopes) {
            Ok(TokenOrRequest::Request {
                request, scope_hash, ..
            }) => {
                #[cfg(not(any(feature = "native-tls", feature = "rustls-tls")))]
                compile_error!(
                    "At least one of native-tls or rustls-tls feature must be enabled to use oauth feature"
                );
                // If both are enabled, we use rustls unlike `Client` because there's no need to support ip v4/6 subject matching.
                // TODO Allow users to choose when both are enabled.
                #[cfg(feature = "rustls-tls")]
                let https = hyper_rustls::HttpsConnector::with_native_roots();
                #[cfg(all(not(feature = "rustls-tls"), feature = "native-tls"))]
                let https = hyper_tls::HttpsConnector::new();
                let client = hyper::Client::builder().build::<_, hyper::Body>(https);

                let res = client
                    .request(request.map(hyper::Body::from))
                    .await
                    .map_err(Error::RequestToken)?;
                // Convert response body to `Vec<u8>` for parsing.
                let (parts, body) = res.into_parts();
                let bytes = hyper::body::to_bytes(body).await.map_err(Error::ConcatBuffers)?;
                let response = http::Response::from_parts(parts, bytes.to_vec());
                match self.access.parse_token_response(scope_hash, response) {
                    Ok(token) => Ok(token),

                    Err(err) => Err(match err {
                        tame_oauth::Error::AuthError(_) | tame_oauth::Error::HttpStatus(_) => {
                            Error::RetrieveCredentials(err)
                        }
                        tame_oauth::Error::Json(e) => Error::ParseToken(e),
                        err => Error::Unknown(err.to_string()),
                    }),
                }
            }

            Ok(TokenOrRequest::Token(token)) => Ok(token),

            Err(err) => match err {
                // Request builder failed.
                tame_oauth::Error::Http(e) => Err(Error::BuildRequest(e)),
                tame_oauth::Error::InvalidRsaKey => Err(Error::InvalidRsaKey(err)),
                tame_oauth::Error::InvalidKeyFormat => Err(Error::InvalidKeyFormat(err)),
                e => Err(Error::Unknown(e.to_string())),
            },
        }
    }
}

const GOOGLE_APPLICATION_CREDENTIALS: &str = "GOOGLE_APPLICATION_CREDENTIALS";

pub(crate) fn gcloud_account_info() -> Result<ServiceAccountInfo, Error> {
    let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
        .map(PathBuf::from)
        .ok_or(Error::MissingGoogleCredentials)?;
    let data = std::fs::read_to_string(path).map_err(Error::LoadCredentials)?;
    ServiceAccountInfo::deserialize(data).map_err(|err| match err {
        tame_oauth::Error::Json(e) => Error::ParseCredentials(e),
        _ => Error::Unknown(err.to_string()),
    })
}
