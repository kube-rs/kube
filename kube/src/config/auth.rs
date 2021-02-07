#[cfg(feature = "oauth")] use std::{env, path::PathBuf};
use std::{process::Command, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use http::header;
use jsonpath_lib::select as jsonpath_select;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[cfg(all(feature = "oauth", feature = "rustls-tls"))]
use hyper_rustls::HttpsConnector;
#[cfg(all(feature = "oauth", feature = "native-tls"))]
use hyper_tls::HttpsConnector;
#[cfg(feature = "oauth")]
use tame_oauth::{
    gcp::{ServiceAccountAccess, ServiceAccountInfo, TokenOrRequest},
    Token,
};

use super::{utils, AuthInfo, AuthProviderConfig, ExecConfig};
#[cfg(feature = "oauth")] use crate::error::OAuthError;
use crate::{
    error::{ConfigError, Error},
    Result,
};

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Authentication {
    None,
    Basic(String),
    Token(String),
    RefreshableToken(RefreshableToken),
}

#[derive(Debug, Clone)]
pub(crate) struct RefreshableToken(pub(crate) Arc<Mutex<(String, DateTime<Utc>, AuthInfo)>>);

impl RefreshableToken {
    pub(crate) async fn to_header(&self) -> Result<header::HeaderValue> {
        let data = &self.0;
        let mut locked_data = data.lock().await;
        // Add some wiggle room onto the current timestamp so we don't get any race
        // conditions where the token expires while we are refreshing
        if Utc::now() + Duration::seconds(60) >= locked_data.1 {
            if let Authentication::RefreshableToken(d) =
                Authentication::from_auth_info(&locked_data.2).await?
            {
                let (new_token, new_expire, new_info) = Arc::try_unwrap(d.0)
                    .expect("Unable to unwrap Arc, this is likely a programming error")
                    .into_inner();
                locked_data.0 = new_token;
                locked_data.1 = new_expire;
                locked_data.2 = new_info;
            } else {
                return Err(ConfigError::UnrefreshableTokenResponse).map_err(Error::from);
            }
        }
        Ok(header::HeaderValue::from_str(&locked_data.0).map_err(ConfigError::InvalidBearerToken)?)
    }
}

impl Authentication {
    pub(crate) async fn to_header(&self) -> Result<Option<header::HeaderValue>> {
        match self {
            Self::None => Ok(None),
            Self::Basic(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBasicAuth)?,
            )),
            Self::Token(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBearerToken)?,
            )),
            Self::RefreshableToken(refreshable) => Ok(Some(refreshable.to_header().await?)),
        }
    }

    /// Loads the authentication header from the credentials available in the kubeconfig. This supports
    /// exec plugins as well as specified in
    /// https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins
    pub(crate) async fn from_auth_info(auth_info: &AuthInfo) -> Result<Self> {
        if let Some(provider) = &auth_info.auth_provider {
            match token_from_provider(provider).await? {
                ProviderToken::GCP(token, Some(expiry)) => {
                    let mut info = auth_info.clone();
                    let mut provider = provider.clone();
                    provider.config.insert("access-token".into(), token.clone());
                    provider.config.insert("expiry".into(), expiry.to_rfc3339());
                    info.auth_provider = Some(provider);
                    return Ok(Self::RefreshableToken(RefreshableToken(Arc::new(Mutex::new((
                        format!("Bearer {}", token),
                        expiry,
                        info,
                    ))))));
                }

                ProviderToken::GCP(token, None) => {
                    return Ok(Self::Token(format!("Bearer {}", token)));
                }
            }
        }

        let (raw_token, expiration) = match &auth_info.token {
            Some(token) => (Some(token.clone()), None),
            None => {
                if let Some(exec) = &auth_info.exec {
                    let creds = auth_exec(exec)?;
                    let status = creds.status.ok_or(ConfigError::ExecPluginFailed)?;
                    let expiration = match status.expiration_timestamp {
                        Some(ts) => Some(
                            ts.parse::<DateTime<Utc>>()
                                .map_err(ConfigError::MalformedTokenExpirationDate)?,
                        ),
                        None => None,
                    };
                    (status.token, expiration)
                } else {
                    (None, None)
                }
            }
        };

        match (
            utils::data_or_file(&raw_token, &auth_info.token_file),
            (&auth_info.username, &auth_info.password),
            expiration,
        ) {
            (Ok(token), _, None) => Ok(Authentication::Token(format!("Bearer {}", token))),
            (Ok(token), _, Some(expire)) => Ok(Authentication::RefreshableToken(RefreshableToken(Arc::new(
                Mutex::new((format!("Bearer {}", token), expire, auth_info.clone())),
            )))),
            (_, (Some(u), Some(p)), _) => {
                let encoded = base64::encode(&format!("{}:{}", u, p));
                Ok(Authentication::Basic(format!("Basic {}", encoded)))
            }
            _ => Ok(Authentication::None),
        }
    }
}

// We need to differentiate providers because the keys/formats to store token expiration differs.
enum ProviderToken {
    // "access-token", "expiry" (RFC3339)
    GCP(String, Option<DateTime<Utc>>),
    // "access-token", "expires-on" (timestamp)
    // Azure(String, Option<DateTime<Utc>>),
}

async fn token_from_provider(provider: &AuthProviderConfig) -> Result<ProviderToken> {
    if provider.name == "gcp" {
        token_from_gcp_provider(provider).await
    } else {
        Err(ConfigError::AuthExec(format!(
            "Authentication with provider {:} not supported",
            provider.name
        ))
        .into())
    }
}

async fn token_from_gcp_provider(provider: &AuthProviderConfig) -> Result<ProviderToken> {
    if let Some(id_token) = provider.config.get("id-token") {
        return Ok(ProviderToken::GCP(id_token.clone(), None));
    }

    // Return cached access token if it's still valid
    if let Some(access_token) = provider.config.get("access-token") {
        if let Some(expiry) = provider.config.get("expiry") {
            let expiry_date = expiry
                .parse::<DateTime<Utc>>()
                .map_err(ConfigError::MalformedTokenExpirationDate)?;
            if Utc::now() + Duration::seconds(60) < expiry_date {
                return Ok(ProviderToken::GCP(access_token.clone(), Some(expiry_date)));
            }
        }
    }

    // Command-based token source
    if let Some(cmd) = provider.config.get("cmd-path") {
        let params = provider.config.get("cmd-args").cloned().unwrap_or_default();

        // TODO splitting args by space is not safe
        let output = Command::new(cmd)
            .args(params.trim().split(' '))
            .output()
            .map_err(|e| ConfigError::AuthExec(format!("Executing {:} failed: {:?}", cmd, e)))?;

        if !output.status.success() {
            return Err(ConfigError::AuthExecRun {
                cmd: format! {"{} {}", cmd, params},
                status: output.status,
                out: output,
            }
            .into());
        }

        if let Some(field) = provider.config.get("token-key") {
            let json_output: serde_json::Value = serde_json::from_slice(&output.stdout)?;
            let token = extract_value(&json_output, &field)?;
            if let Some(field) = provider.config.get("expiry-key") {
                let expiry = extract_value(&json_output, &field)?;
                let expiry = expiry
                    .parse::<DateTime<Utc>>()
                    .map_err(ConfigError::MalformedTokenExpirationDate)?;
                return Ok(ProviderToken::GCP(token, Some(expiry)));
            } else {
                return Ok(ProviderToken::GCP(token, None));
            }
        } else {
            let token = std::str::from_utf8(&output.stdout)
                .map_err(|e| ConfigError::AuthExec(format!("Result is not a string {:?} ", e)))?
                .to_owned();
            return Ok(ProviderToken::GCP(token, None));
        }
    }

    // Google Application Credentials-based token source
    #[cfg(feature = "oauth")]
    {
        let token_res = if let Some(scopes) = provider.config.get("scopes") {
            request_gcp_token(&scopes.split(',').collect::<Vec<_>>()).await?
        } else {
            request_gcp_token(&[
                "https://www.googleapis.com/auth/cloud-platform",
                "https://www.googleapis.com/auth/userinfo.email",
            ])
            .await?
        };
        let expiry_date = token_res.expiry_date();
        Ok(ProviderToken::GCP(token_res.access_token, Some(expiry_date)))
    }
    #[cfg(not(feature = "oauth"))]
    {
        Err(ConfigError::AuthExec(
            "Enable oauth feature to use Google Application Credentials-based token source".into(),
        )
        .into())
    }
}

fn extract_value(json: &serde_json::Value, path: &str) -> Result<String> {
    let pure_path = path.trim_matches(|c| c == '"' || c == '{' || c == '}');
    match jsonpath_select(json, &format!("${}", pure_path)) {
        Ok(v) if !v.is_empty() => {
            if let serde_json::Value::String(res) = v[0] {
                Ok(res.clone())
            } else {
                Err(ConfigError::AuthExec(format!("Target value at {:} is not a string", pure_path)).into())
            }
        }

        Err(e) => Err(ConfigError::AuthExec(format!("Could not extract JSON value: {:}", e)).into()),

        _ => Err(ConfigError::AuthExec(format!("Target value {:} not found", pure_path)).into()),
    }
}

/// ExecCredentials is used by exec-based plugins to communicate credentials to
/// HTTP transports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecCredential {
    pub kind: Option<String>,
    #[serde(rename = "apiVersion")]
    pub api_version: Option<String>,
    pub spec: Option<ExecCredentialSpec>,
    pub status: Option<ExecCredentialStatus>,
}

/// ExecCredenitalSpec holds request and runtime specific information provided
/// by transport.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecCredentialSpec {}

/// ExecCredentialStatus holds credentials for the transport to use.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecCredentialStatus {
    #[serde(rename = "expirationTimestamp")]
    pub expiration_timestamp: Option<String>,
    pub token: Option<String>,
    #[serde(rename = "clientCertificateData")]
    pub client_certificate_data: Option<String>,
    #[serde(rename = "clientKeyData")]
    pub client_key_data: Option<String>,
}

fn auth_exec(auth: &ExecConfig) -> Result<ExecCredential, ConfigError> {
    let mut cmd = Command::new(&auth.command);
    if let Some(args) = &auth.args {
        cmd.args(args);
    }
    if let Some(env) = &auth.env {
        let envs = env
            .iter()
            .flat_map(|env| match (env.get("name"), env.get("value")) {
                (Some(name), Some(value)) => Some((name, value)),
                _ => None,
            });
        cmd.envs(envs);
    }
    let out = cmd.output().map_err(ConfigError::AuthExecStart)?;
    if !out.status.success() {
        return Err(ConfigError::AuthExecRun {
            cmd: format!("{:?}", cmd),
            status: out.status,
            out,
        });
    }
    let creds = serde_json::from_slice(&out.stdout).map_err(ConfigError::AuthExecParse)?;

    Ok(creds)
}

#[cfg(feature = "oauth")]
pub async fn request_gcp_token<'a, S, I>(scopes: I) -> Result<Token>
where
    S: AsRef<str> + 'a,
    I: IntoIterator<Item = &'a S>,
{
    let info = gcloud_account_info()?;
    let access = ServiceAccountAccess::new(info).map_err(OAuthError::InvalidKeyFormat)?;
    match access.get_token(scopes) {
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
                .map_err(OAuthError::RequestToken)?;
            // Convert response body to `Vec<u8>` for parsing.
            let (parts, body) = res.into_parts();
            let bytes = hyper::body::to_bytes(body).await?;
            let response = http::Response::from_parts(parts, bytes.to_vec());
            match access.parse_token_response(scope_hash, response) {
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

        // ServiceAccountAccess was just created, so it's impossible to have cached token.
        Ok(TokenOrRequest::Token(_)) => unreachable!(),

        Err(err) => match err {
            // Request builder failed.
            tame_oauth::Error::Http(e) => Err(Error::HttpError(e)),
            tame_oauth::Error::InvalidRsaKey => Err(OAuthError::InvalidRsaKey(err).into()),
            tame_oauth::Error::InvalidKeyFormat => Err(OAuthError::InvalidKeyFormat(err).into()),
            e => Err(OAuthError::Unknown(e.to_string()).into()),
        },
    }
}

#[cfg(feature = "oauth")]
const GOOGLE_APPLICATION_CREDENTIALS: &str = "GOOGLE_APPLICATION_CREDENTIALS";

#[cfg(feature = "oauth")]
fn gcloud_account_info() -> Result<ServiceAccountInfo, ConfigError> {
    let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
        .map(PathBuf::from)
        .ok_or(OAuthError::MissingGoogleCredentials)?;
    let data = std::fs::read_to_string(path).map_err(OAuthError::LoadCredentials)?;
    ServiceAccountInfo::deserialize(data).map_err(|err| match err {
        tame_oauth::Error::Json(e) => OAuthError::ParseCredentials(e).into(),
        _ => OAuthError::Unknown(err.to_string()).into(),
    })
}

#[cfg(test)]
mod test {
    use crate::config::Kubeconfig;

    use super::*;
    #[tokio::test]
    async fn exec_auth_command() -> Result<()> {
        let expiry = (Utc::now() + Duration::seconds(60 * 60)).to_rfc3339();
        let test_file = format!(
            r#"
        apiVersion: v1
        clusters:
        - cluster:
            certificate-authority-data: XXXXXXX
            server: https://36.XXX.XXX.XX
          name: generic-name
        contexts:
        - context:
            cluster: generic-name
            user: generic-name
          name: generic-name
        current-context: generic-name
        kind: Config
        preferences: {{}}
        users:
        - name: generic-name
          user:
            auth-provider:
              config:
                cmd-args: '{{"something": "else", "credential": {{"access_token": "my_token", "token_expiry": "{expiry}"}}}}'
                cmd-path: echo
                expiry-key: '{{.credential.token_expiry}}'
                token-key: '{{.credential.access_token}}'
              name: gcp
        "#,
            expiry = expiry
        );

        let mut config: Kubeconfig = serde_yaml::from_str(&test_file).map_err(ConfigError::ParseYaml)?;
        let auth_info = &mut config.auth_infos[0].auth_info;
        match Authentication::from_auth_info(&auth_info).await {
            Ok(Authentication::RefreshableToken(refreshable)) => {
                let (token, _expire, info) = Arc::try_unwrap(refreshable.0).unwrap().into_inner();
                assert_eq!(token, "Bearer my_token".to_owned());
                let config = info.auth_provider.unwrap().config;
                assert_eq!(config.get("access-token"), Some(&"my_token".to_owned()));
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
