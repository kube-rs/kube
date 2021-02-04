use std::{
    convert::{TryFrom, TryInto},
    process::Command,
    sync::Arc,
};

use chrono::{DateTime, Utc};
use http::header;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::{utils, AuthInfo, ExecConfig};
use crate::error::ConfigError;

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum Authentication {
    None,
    Basic(String),
    Token(String),
    RefreshableToken(Arc<Mutex<(String, DateTime<Utc>)>>, AuthInfo),
}

impl Authentication {
    pub(crate) async fn to_header(&self) -> Result<Option<header::HeaderValue>, ConfigError> {
        match self {
            Self::None => Ok(None),
            Self::Basic(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBasicAuth)?,
            )),
            Self::Token(value) => Ok(Some(
                header::HeaderValue::from_str(value).map_err(ConfigError::InvalidBearerToken)?,
            )),
            Self::RefreshableToken(data, auth_info) => {
                let mut locked_data = data.lock().await;
                // Add some wiggle room onto the current timestamp so we don't get any race
                // conditions where the token expires while we are refreshing
                if chrono::Utc::now() + chrono::Duration::seconds(60) >= locked_data.1 {
                    if let Authentication::RefreshableToken(d, _) = auth_info.try_into()? {
                        let (new_token, new_expire) = Arc::try_unwrap(d)
                            .expect("Unable to unwrap Arc, this is likely a programming error")
                            .into_inner();
                        locked_data.0 = new_token;
                        locked_data.1 = new_expire;
                    } else {
                        return Err(ConfigError::UnrefreshableTokenResponse);
                    }
                }
                Ok(Some(
                    header::HeaderValue::from_str(&locked_data.0).map_err(ConfigError::InvalidBearerToken)?,
                ))
            }
        }
    }
}

impl TryFrom<&AuthInfo> for Authentication {
    type Error = ConfigError;

    /// Loads the authentication header from the credentials available in the kubeconfig. This supports
    /// exec plugins as well as specified in
    /// https://kubernetes.io/docs/reference/access-authn-authz/authentication/#client-go-credential-plugins
    fn try_from(auth_info: &AuthInfo) -> Result<Self, Self::Error> {
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
            (Ok(token), _, Some(expire)) => Ok(Authentication::RefreshableToken(
                Arc::new(Mutex::new((format!("Bearer {}", token), expire))),
                auth_info.clone(),
            )),
            (_, (Some(u), Some(p)), _) => {
                let encoded = base64::encode(&format!("{}:{}", u, p));
                Ok(Authentication::Basic(format!("Basic {}", encoded)))
            }
            _ => Ok(Authentication::None),
        }
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
