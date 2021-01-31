use std::{env, path::PathBuf};

use tame_oauth::{
    gcp::{ServiceAccountAccess, ServiceAccountInfo, TokenOrRequest},
    Error as OauthError, Token,
};

#[cfg(feature = "rustls-tls")] use hyper_rustls::HttpsConnector;
#[cfg(feature = "native-tls")] use hyper_tls::HttpsConnector;

use crate::{
    error::{ConfigError, Error},
    Result,
};

const GOOGLE_APPLICATION_CREDENTIALS: &str = "GOOGLE_APPLICATION_CREDENTIALS";

pub async fn get_token() -> Result<Token> {
    let info = get_account_info()?;
    let access = ServiceAccountAccess::new(info).map_err(ConfigError::OAuth2InvalidKeyFormat)?;
    match access.get_token(&["https://www.googleapis.com/auth/cloud-platform"]) {
        Ok(TokenOrRequest::Request {
            request, scope_hash, ..
        }) => {
            #[cfg(feature = "native-tls")]
            let https = HttpsConnector::new();
            #[cfg(feature = "rustls-tls")]
            let https = HttpsConnector::with_native_roots();
            let client = hyper::Client::builder().build::<_, hyper::Body>(https);

            let res = client
                .request(request.map(hyper::Body::from))
                .await
                .map_err(ConfigError::OAuth2RequestToken)?;
            // Convert response body to `Vec<u8>` for parsing.
            let (parts, body) = res.into_parts();
            let bytes = hyper::body::to_bytes(body).await?;
            let response = http::Response::from_parts(parts, bytes.to_vec());
            match access.parse_token_response(scope_hash, response) {
                Ok(token) => Ok(token),

                Err(err) => match err {
                    OauthError::AuthError(_) | OauthError::HttpStatus(_) => {
                        Err(ConfigError::OAuth2RetrieveCredentials(err)).map_err(Error::from)
                    }
                    OauthError::Json(e) => Err(ConfigError::OAuth2ParseToken(e)).map_err(Error::from),
                    err => Err(ConfigError::OAuth2Unknown(err.to_string())).map_err(Error::from),
                },
            }
        }

        // ServiceAccountAccess was just created, so it's impossible to have cached token.
        Ok(TokenOrRequest::Token(_)) => unreachable!(),

        Err(err) => match err {
            // Request builder failed.
            OauthError::Http(e) => Err(Error::HttpError(e)),
            OauthError::InvalidRsaKey => Err(ConfigError::OAuth2InvalidRsaKey(err)).map_err(Error::from),
            OauthError::InvalidKeyFormat => {
                Err(ConfigError::OAuth2InvalidKeyFormat(err)).map_err(Error::from)
            }
            e => Err(ConfigError::OAuth2Unknown(e.to_string())).map_err(Error::from),
        },
    }
}

fn get_account_info() -> Result<ServiceAccountInfo, ConfigError> {
    let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
        .map(PathBuf::from)
        .ok_or(ConfigError::MissingGoogleCredentials)?;
    let data = std::fs::read_to_string(path).map_err(ConfigError::OAuth2LoadCredentials)?;
    ServiceAccountInfo::deserialize(data).map_err(|err| match err {
        OauthError::Json(e) => ConfigError::OAuth2ParseCredentials(e),
        _ => ConfigError::OAuth2Unknown(err.to_string()),
    })
}
