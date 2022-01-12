use std::{path::PathBuf, process::Command, sync::Arc};

use chrono::{DateTime, Duration, Utc};
use futures::future::BoxFuture;
use http::{
    header::{InvalidHeaderValue, AUTHORIZATION},
    HeaderValue, Request,
};
use jsonpath_lib::select as jsonpath_select;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;
use tower::{filter::AsyncPredicate, BoxError};

use crate::config::{AuthInfo, AuthProviderConfig, ExecConfig};

#[cfg(feature = "oauth")] mod oauth;
#[cfg(feature = "oauth")] pub use oauth::Error as OAuthError;

#[derive(Error, Debug)]
/// Client auth errors
pub enum Error {
    /// Invalid basic auth
    #[error("invalid basic auth: {0}")]
    InvalidBasicAuth(#[source] InvalidHeaderValue),

    /// Invalid bearer token
    #[error("invalid bearer token: {0}")]
    InvalidBearerToken(#[source] InvalidHeaderValue),

    /// Tried to refresh a token and got a non-refreshable token response
    #[error("tried to refresh a token and got a non-refreshable token response")]
    UnrefreshableTokenResponse,

    /// Exec plugin response did not contain a status
    #[error("exec-plugin response did not contain a status")]
    ExecPluginFailed,

    /// Malformed token expiration date
    #[error("malformed token expiration date: {0}")]
    MalformedTokenExpirationDate(#[source] chrono::ParseError),

    /// Failed to start auth exec
    #[error("unable to run auth exec: {0}")]
    AuthExecStart(#[source] std::io::Error),

    /// Failed to run auth exec command
    #[error("auth exec command '{cmd}' failed with status {status}: {out:?}")]
    AuthExecRun {
        /// The failed command
        cmd: String,
        /// The exit status or exit code of the failed command
        status: std::process::ExitStatus,
        /// Stdout/Stderr of the failed command
        out: std::process::Output,
    },

    /// Failed to parse auth exec output
    #[error("failed to parse auth exec output: {0}")]
    AuthExecParse(#[source] serde_json::Error),

    /// Failed to exec auth
    #[error("failed exec auth: {0}")]
    AuthExec(String),

    /// Failed to read token file
    #[error("failed to read token file '{1:?}': {0}")]
    ReadTokenFile(#[source] std::io::Error, PathBuf),

    /// Failed to parse token-key
    #[error("failed to parse token-key")]
    ParseTokenKey(#[source] serde_json::Error),

    /// OAuth error
    #[cfg(feature = "oauth")]
    #[cfg_attr(docsrs, doc(cfg(feature = "oauth")))]
    #[error("failed OAuth: {0}")]
    OAuth(#[source] OAuthError),
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Auth {
    None,
    Basic(String, SecretString),
    Bearer(SecretString),
    RefreshableToken(RefreshableToken),
}

// See https://github.com/kubernetes/kubernetes/tree/master/staging/src/k8s.io/client-go/plugin/pkg/client/auth
// for the list of auth-plugins supported by client-go.
// We currently support the following:
// - exec
// - gcp: command based token source (exec)
// - gcp: application credential based token source (requires `oauth` feature)
//
// Note that the visibility must be `pub` for `impl Layer for AuthLayer`, but this is not exported from the crate.
// It's not accessible from outside and not shown on docs.
#[derive(Debug, Clone)]
pub enum RefreshableToken {
    Exec(Arc<Mutex<(SecretString, DateTime<Utc>, AuthInfo)>>),
    #[cfg(feature = "oauth")]
    GcpOauth(Arc<Mutex<oauth::Gcp>>),
}

// For use with `AsyncFilterLayer` to add `Authorization` header with a refreshed token.
impl<B> AsyncPredicate<Request<B>> for RefreshableToken
where
    B: http_body::Body + Send + 'static,
{
    type Future = BoxFuture<'static, Result<Request<B>, BoxError>>;
    type Request = Request<B>;

    fn check(&mut self, mut request: Self::Request) -> Self::Future {
        let refreshable = self.clone();
        Box::pin(async move {
            refreshable.to_header().await.map_err(Into::into).map(|value| {
                request.headers_mut().insert(AUTHORIZATION, value);
                request
            })
        })
    }
}

impl RefreshableToken {
    async fn to_header(&self) -> Result<HeaderValue, Error> {
        match self {
            RefreshableToken::Exec(data) => {
                let mut locked_data = data.lock().await;
                // Add some wiggle room onto the current timestamp so we don't get any race
                // conditions where the token expires while we are refreshing
                if Utc::now() + Duration::seconds(60) >= locked_data.1 {
                    match Auth::try_from(&locked_data.2)? {
                        Auth::None | Auth::Basic(_, _) | Auth::Bearer(_) => {
                            return Err(Error::UnrefreshableTokenResponse);
                        }

                        Auth::RefreshableToken(RefreshableToken::Exec(d)) => {
                            let (new_token, new_expire, new_info) = Arc::try_unwrap(d)
                                .expect("Unable to unwrap Arc, this is likely a programming error")
                                .into_inner();
                            locked_data.0 = new_token;
                            locked_data.1 = new_expire;
                            locked_data.2 = new_info;
                        }

                        // Unreachable because the token source does not change
                        #[cfg(feature = "oauth")]
                        Auth::RefreshableToken(RefreshableToken::GcpOauth(_)) => unreachable!(),
                    }
                }

                let mut value = HeaderValue::try_from(format!("Bearer {}", &locked_data.0.expose_secret()))
                    .map_err(Error::InvalidBearerToken)?;
                value.set_sensitive(true);
                Ok(value)
            }

            #[cfg(feature = "oauth")]
            RefreshableToken::GcpOauth(data) => {
                let gcp_oauth = data.lock().await;
                let token = (*gcp_oauth).token().await.map_err(Error::OAuth)?;
                let mut value = HeaderValue::try_from(format!("Bearer {}", &token.access_token))
                    .map_err(Error::InvalidBearerToken)?;
                value.set_sensitive(true);
                Ok(value)
            }
        }
    }
}

impl TryFrom<&AuthInfo> for Auth {
    type Error = Error;

    /// Loads the authentication header from the credentials available in the kubeconfig. This supports
    /// exec plugins as well as specified in
    /// https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins
    fn try_from(auth_info: &AuthInfo) -> Result<Self, Self::Error> {
        if let Some(provider) = &auth_info.auth_provider {
            match token_from_provider(provider)? {
                ProviderToken::Oidc(token) => {
                    return Ok(Self::Bearer(SecretString::from(token)));
                }

                ProviderToken::GcpCommand(token, Some(expiry)) => {
                    let mut info = auth_info.clone();
                    let mut provider = provider.clone();
                    provider.config.insert("access-token".into(), token.clone());
                    provider.config.insert("expiry".into(), expiry.to_rfc3339());
                    info.auth_provider = Some(provider);
                    return Ok(Self::RefreshableToken(RefreshableToken::Exec(Arc::new(
                        Mutex::new((SecretString::from(token), expiry, info)),
                    ))));
                }

                ProviderToken::GcpCommand(token, None) => {
                    return Ok(Self::Bearer(SecretString::from(token)));
                }

                #[cfg(feature = "oauth")]
                ProviderToken::GcpOauth(gcp) => {
                    return Ok(Self::RefreshableToken(RefreshableToken::GcpOauth(Arc::new(
                        Mutex::new(gcp),
                    ))));
                }
            }
        }

        if let (Some(u), Some(p)) = (&auth_info.username, &auth_info.password) {
            return Ok(Self::Basic(u.to_owned(), p.to_owned()));
        }

        let (raw_token, expiration) = match &auth_info.token {
            Some(token) => (Some(token.clone()), None),
            None => {
                if let Some(exec) = &auth_info.exec {
                    let creds = auth_exec(exec)?;
                    let status = creds.status.ok_or(Error::ExecPluginFailed)?;
                    let expiration = status
                        .expiration_timestamp
                        .map(|ts| ts.parse())
                        .transpose()
                        .map_err(Error::MalformedTokenExpirationDate)?;
                    (status.token.map(SecretString::from), expiration)
                } else if let Some(file) = &auth_info.token_file {
                    let token = std::fs::read_to_string(&file)
                        .map_err(|source| Error::ReadTokenFile(source, file.into()))?;
                    (Some(SecretString::from(token)), None)
                } else {
                    (None, None)
                }
            }
        };

        match (raw_token, expiration) {
            (Some(token), None) => Ok(Self::Bearer(token)),
            (Some(token), Some(expire)) => Ok(Self::RefreshableToken(RefreshableToken::Exec(Arc::new(
                Mutex::new((token, expire, auth_info.clone())),
            )))),
            _ => Ok(Self::None),
        }
    }
}

// We need to differentiate providers because the keys/formats to store token expiration differs.
enum ProviderToken {
    Oidc(String),
    // "access-token", "expiry" (RFC3339)
    GcpCommand(String, Option<DateTime<Utc>>),
    #[cfg(feature = "oauth")]
    GcpOauth(oauth::Gcp),
    // "access-token", "expires-on" (timestamp)
    // Azure(String, Option<DateTime<Utc>>),
}

fn token_from_provider(provider: &AuthProviderConfig) -> Result<ProviderToken, Error> {
    match provider.name.as_ref() {
        "oidc" => token_from_oidc_provider(provider),
        "gcp" => token_from_gcp_provider(provider),
        _ => Err(Error::AuthExec(format!(
            "Authentication with provider {:} not supported",
            provider.name
        ))),
    }
}

fn token_from_oidc_provider(provider: &AuthProviderConfig) -> Result<ProviderToken, Error> {
    match provider.config.get("id-token") {
        Some(id_token) => Ok(ProviderToken::Oidc(id_token.clone())),
        None => Err(Error::AuthExec(
            "No id-token for oidc Authentication provider".into(),
        )),
    }
}

fn token_from_gcp_provider(provider: &AuthProviderConfig) -> Result<ProviderToken, Error> {
    if let Some(id_token) = provider.config.get("id-token") {
        return Ok(ProviderToken::GcpCommand(id_token.clone(), None));
    }

    // Return cached access token if it's still valid
    if let Some(access_token) = provider.config.get("access-token") {
        if let Some(expiry) = provider.config.get("expiry") {
            let expiry_date = expiry
                .parse::<DateTime<Utc>>()
                .map_err(Error::MalformedTokenExpirationDate)?;
            if Utc::now() + Duration::seconds(60) < expiry_date {
                return Ok(ProviderToken::GcpCommand(access_token.clone(), Some(expiry_date)));
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
            .map_err(|e| Error::AuthExec(format!("Executing {:} failed: {:?}", cmd, e)))?;

        if !output.status.success() {
            return Err(Error::AuthExecRun {
                cmd: format!("{} {}", cmd, params),
                status: output.status,
                out: output,
            });
        }

        if let Some(field) = provider.config.get("token-key") {
            let json_output: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(Error::ParseTokenKey)?;
            let token = extract_value(&json_output, field)?;
            if let Some(field) = provider.config.get("expiry-key") {
                let expiry = extract_value(&json_output, field)?;
                let expiry = expiry
                    .parse::<DateTime<Utc>>()
                    .map_err(Error::MalformedTokenExpirationDate)?;
                return Ok(ProviderToken::GcpCommand(token, Some(expiry)));
            } else {
                return Ok(ProviderToken::GcpCommand(token, None));
            }
        } else {
            let token = std::str::from_utf8(&output.stdout)
                .map_err(|e| Error::AuthExec(format!("Result is not a string {:?} ", e)))?
                .to_owned();
            return Ok(ProviderToken::GcpCommand(token, None));
        }
    }

    // Google Application Credentials-based token source
    #[cfg(feature = "oauth")]
    {
        Ok(ProviderToken::GcpOauth(
            oauth::Gcp::default_credentials_with_scopes(provider.config.get("scopes"))
                .map_err(Error::OAuth)?,
        ))
    }
    #[cfg(not(feature = "oauth"))]
    {
        Err(Error::AuthExec(
            "Enable oauth feature to use Google Application Credentials-based token source".into(),
        ))
    }
}

fn extract_value(json: &serde_json::Value, path: &str) -> Result<String, Error> {
    let pure_path = path.trim_matches(|c| c == '"' || c == '{' || c == '}');
    match jsonpath_select(json, &format!("${}", pure_path)) {
        Ok(v) if !v.is_empty() => {
            if let serde_json::Value::String(res) = v[0] {
                Ok(res.clone())
            } else {
                Err(Error::AuthExec(format!(
                    "Target value at {:} is not a string",
                    pure_path
                )))
            }
        }

        Err(e) => Err(Error::AuthExec(format!("Could not extract JSON value: {:}", e))),

        _ => Err(Error::AuthExec(format!("Target value {:} not found", pure_path))),
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

fn auth_exec(auth: &ExecConfig) -> Result<ExecCredential, Error> {
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
    let out = cmd.output().map_err(Error::AuthExecStart)?;
    if !out.status.success() {
        return Err(Error::AuthExecRun {
            cmd: format!("{:?}", cmd),
            status: out.status,
            out,
        });
    }
    let creds = serde_json::from_slice(&out.stdout).map_err(Error::AuthExecParse)?;

    Ok(creds)
}

#[cfg(test)]
mod test {
    use crate::config::Kubeconfig;

    use super::*;
    #[tokio::test]
    async fn exec_auth_command() -> Result<(), Error> {
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

        let config: Kubeconfig = serde_yaml::from_str(&test_file).unwrap();
        let auth_info = &config.auth_infos[0].auth_info;
        match Auth::try_from(auth_info).unwrap() {
            Auth::RefreshableToken(RefreshableToken::Exec(refreshable)) => {
                let (token, _expire, info) = Arc::try_unwrap(refreshable).unwrap().into_inner();
                assert_eq!(token.expose_secret(), &"my_token".to_owned());
                let config = info.auth_provider.unwrap().config;
                assert_eq!(config.get("access-token"), Some(&"my_token".to_owned()));
            }
            _ => unreachable!(),
        }
        Ok(())
    }
}
