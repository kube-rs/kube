use std::{env, path::PathBuf};

use tame_oauth::{
    gcp::{ServiceAccountAccess, ServiceAccountInfo, TokenOrRequest},
    Token,
};

use crate::{
    error::{ConfigError, OAuthError},
    Error, Result,
};

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
    pub(crate) fn from_env_and_scopes(scopes: Option<&String>) -> Result<Self> {
        const DEFAULT_SCOPES: &str =
            "https://www.googleapis.com/auth/cloud-platform,https://www.googleapis.com/auth/userinfo.email";
        // Initialize ServiceAccountAccess so we can request later when needed.
        let info = gcloud_account_info()?;
        let access = ServiceAccountAccess::new(info).map_err(OAuthError::InvalidKeyFormat)?;
        let scopes = scopes
            .map(String::to_owned)
            .unwrap_or_else(|| DEFAULT_SCOPES.to_owned())
            .split(',')
            .map(str::to_owned)
            .collect::<Vec<_>>();
        Ok(Self::new(access, scopes))
    }

    pub async fn token(&self) -> Result<Token> {
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
                    .map_err(OAuthError::RequestToken)?;
                // Convert response body to `Vec<u8>` for parsing.
                let (parts, body) = res.into_parts();
                let bytes = hyper::body::to_bytes(body).await?;
                let response = http::Response::from_parts(parts, bytes.to_vec());
                match self.access.parse_token_response(scope_hash, response) {
                    Ok(token) => Ok(token),

                    Err(err) => match err {
                        tame_oauth::Error::AuthError(_) | tame_oauth::Error::HttpStatus(_) => {
                            Err(OAuthError::RetrieveCredentials(err).into())
                        }
                        tame_oauth::Error::Json(e) => Err(OAuthError::ParseToken(e).into()),
                        err => Err(OAuthError::Unknown(err.to_string()).into()),
                    },
                }
            }

            Ok(TokenOrRequest::Token(token)) => Ok(token),

            Err(err) => match err {
                // Request builder failed.
                tame_oauth::Error::Http(e) => Err(Error::HttpError(e)),
                tame_oauth::Error::InvalidRsaKey => Err(OAuthError::InvalidRsaKey(err).into()),
                tame_oauth::Error::InvalidKeyFormat => Err(OAuthError::InvalidKeyFormat(err).into()),
                e => Err(OAuthError::Unknown(e.to_string()).into()),
            },
        }
    }
}

const GOOGLE_APPLICATION_CREDENTIALS: &str = "GOOGLE_APPLICATION_CREDENTIALS";

pub(crate) fn gcloud_account_info() -> Result<ServiceAccountInfo, ConfigError> {
    let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
        .map(PathBuf::from)
        .ok_or(OAuthError::MissingGoogleCredentials)?;
    let data = std::fs::read_to_string(path).map_err(OAuthError::LoadCredentials)?;
    ServiceAccountInfo::deserialize(data).map_err(|err| match err {
        tame_oauth::Error::Json(e) => OAuthError::ParseCredentials(e).into(),
        _ => OAuthError::Unknown(err.to_string()).into(),
    })
}
