use std::{
    convert::{TryFrom, TryInto},
    process::Command,
    sync::Arc,
};

use chrono::{DateTime, Utc};
use http::header;
use jsonpath_lib::select as jsonpath_select;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use super::{utils, AuthInfo, AuthProviderConfig, ExecConfig};
use crate::{error::ConfigError, oauth2, Result};

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

pub(crate) async fn token_from_provider(provider: &AuthProviderConfig) -> Result<Option<String>> {
    let mut token = None;
    if let Some(access_token) = provider.config.get("access-token") {
        token = Some(access_token.clone());
        if utils::is_expired(&provider.config["expiry"]) {
            // TODO This is GCP only. Check provider.name == "gcp"?
            let token_res = oauth2::get_token().await?;
            token = Some(token_res.access_token);
        }
    }

    if let Some(id_token) = provider.config.get("id-token") {
        token = Some(id_token.clone());
    }

    if token.is_none() {
        if let Some(cmd) = provider.config.get("cmd-path") {
            let params = provider.config.get("cmd-args").cloned().unwrap_or_default();

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
                let pure_path = field.trim_matches(|c| c == '"' || c == '{' || c == '}');
                let json_output: serde_json::Value = serde_json::from_slice(&output.stdout)?;
                match jsonpath_select(&json_output, &format!("${}", pure_path)) {
                    Ok(v) if !v.is_empty() => {
                        if let serde_json::Value::String(res) = v[0] {
                            token = Some(res.clone());
                        } else {
                            return Err(ConfigError::AuthExec(format!(
                                "Target value at {:} is not a string",
                                pure_path
                            ))
                            .into());
                        }
                    }
                    Err(e) => {
                        return Err(
                            ConfigError::AuthExec(format!("Could not extract JSON value: {:}", e)).into(),
                        );
                    }
                    _ => {
                        return Err(
                            ConfigError::AuthExec(format!("Target value {:} not found", pure_path)).into(),
                        );
                    }
                };
            } else {
                token = Some(
                    std::str::from_utf8(&output.stdout)
                        .map_err(|e| ConfigError::AuthExec(format!("Result is not a string {:?} ", e)))?
                        .to_owned(),
                );
            }
        } else {
            return Err(ConfigError::AuthExec(format!(
                "no token or command provided. Authoring mechanism {:} not supported",
                provider.name
            ))
            .into());
        }
    }

    Ok(token)
}

#[cfg(test)]
mod test {
    use crate::config::Kubeconfig;

    use super::*;
    #[tokio::test]
    async fn exec_auth_command() -> Result<()> {
        let test_file = "
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
        preferences: {}
        users:
        - name: generic-name
          user:
            auth-provider:
              config:
                cmd-args: '{\"something\": \"else\", \"credential\" : {\"access_token\" : \"my_token\"} }'
                cmd-path: echo
                expiry-key: '{.credential.token_expiry}'
                token-key: '{.credential.access_token}'
              name: gcp
        ";

        let mut config: Kubeconfig = serde_yaml::from_str(test_file).map_err(ConfigError::ParseYaml)?;
        let auth_info = &mut config.auth_infos[0].auth_info;
        assert!(auth_info.token.is_none());
        if let Some(provider) = &auth_info.auth_provider {
            auth_info.token = token_from_provider(provider).await?;
        }
        assert_eq!(auth_info.token, Some("my_token".to_owned()));

        Ok(())
    }
}
