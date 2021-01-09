#![allow(missing_docs)]

use std::{collections::HashMap, fs::File, path::Path};

use crate::{config::utils, error::ConfigError, oauth2, Result};

use serde::{Deserialize, Serialize};

use jsonpath_lib::select as jsonpath_select;

/// [`Kubeconfig`] represents information on how to connect to a remote Kubernetes cluster
/// that is normally stored in `~/.kube/config`
///
/// This type (and its children) are exposed for convenience only.
/// Please load a [`Config`][crate::Config] object for use with a [`Client`][crate::Client]
/// which will read and parse the kubeconfig file
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kubeconfig {
    pub kind: Option<String>,
    #[serde(rename = "apiVersion")]
    pub api_version: Option<String>,
    pub preferences: Option<Preferences>,
    pub clusters: Vec<NamedCluster>,
    #[serde(rename = "users")]
    pub auth_infos: Vec<NamedAuthInfo>,
    pub contexts: Vec<NamedContext>,
    #[serde(rename = "current-context")]
    pub current_context: String,
    pub extensions: Option<Vec<NamedExtension>>,
}

/// Preferences stores extensions for cli.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferences {
    pub colors: Option<bool>,
    pub extensions: Option<Vec<NamedExtension>>,
}

/// NamedExtention associates name with extension.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedExtension {
    pub name: String,
    pub extension: String,
}

/// NamedCluster associates name with cluster.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedCluster {
    pub name: String,
    pub cluster: Cluster,
}

/// Cluster stores information to connect Kubernetes cluster.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Cluster {
    pub server: String,
    #[serde(rename = "insecure-skip-tls-verify")]
    pub insecure_skip_tls_verify: Option<bool>,
    #[serde(rename = "certificate-authority")]
    pub certificate_authority: Option<String>,
    #[serde(rename = "certificate-authority-data")]
    pub certificate_authority_data: Option<String>,
}

/// NamedAuthInfo associates name with authentication.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedAuthInfo {
    pub name: String,
    #[serde(rename = "user")]
    pub auth_info: AuthInfo,
}

/// AuthInfo stores information to tell cluster who you are.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthInfo {
    pub username: Option<String>,
    pub password: Option<String>,

    pub token: Option<String>,
    #[serde(rename = "tokenFile")]
    pub token_file: Option<String>,

    #[serde(rename = "client-certificate")]
    pub client_certificate: Option<String>,
    #[serde(rename = "client-certificate-data")]
    pub client_certificate_data: Option<String>,

    #[serde(rename = "client-key")]
    pub client_key: Option<String>,
    #[serde(rename = "client-key-data")]
    pub client_key_data: Option<String>,

    #[serde(rename = "as")]
    pub impersonate: Option<String>,
    #[serde(rename = "as-groups")]
    pub impersonate_groups: Option<Vec<String>>,

    #[serde(rename = "auth-provider")]
    pub auth_provider: Option<AuthProviderConfig>,

    pub exec: Option<ExecConfig>,
}

/// AuthProviderConfig stores auth for specified cloud provider.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthProviderConfig {
    pub name: String,
    pub config: HashMap<String, String>,
}

/// ExecConfig stores credential-plugin configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecConfig {
    #[serde(rename = "apiVersion")]
    pub api_version: Option<String>,
    pub args: Option<Vec<String>>,
    pub command: String,
    pub env: Option<Vec<HashMap<String, String>>>,
}

/// NamedContext associates name with context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NamedContext {
    pub name: String,
    pub context: Context,
}

/// Context stores tuple of cluster and user information.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Context {
    pub cluster: String,
    pub user: String,
    pub namespace: Option<String>,
    pub extensions: Option<Vec<NamedExtension>>,
}

/// Some helpers on the raw Config object are exposed for people needing to parse it
impl Kubeconfig {
    /// Read a Config from an arbitrary location
    pub fn read_from<P: AsRef<Path>>(path: P) -> Result<Kubeconfig> {
        let f = File::open(&path).map_err(|source| ConfigError::ReadFile {
            path: path.as_ref().into(),
            source,
        })?;
        let config = serde_yaml::from_reader(f).map_err(ConfigError::ParseYaml)?;
        Ok(config)
    }

    /// Read a Config from the default location
    pub fn read() -> Result<Kubeconfig> {
        let path = utils::find_kubeconfig()?;
        Self::read_from(path)
    }
}

impl Cluster {
    pub(crate) fn load_certificate_authority(&self) -> Result<Option<Vec<u8>>> {
        if self.certificate_authority.is_none() && self.certificate_authority_data.is_none() {
            return Ok(None);
        }
        let res =
            utils::data_or_file_with_base64(&self.certificate_authority_data, &self.certificate_authority)?;
        Ok(Some(res))
    }
}

impl AuthInfo {
    pub(crate) async fn load_gcp(&mut self) -> Result<()> {
        match &self.auth_provider {
            Some(provider) => {
                if let Some(access_token) = provider.config.get("access-token") {
                    self.token = Some(access_token.clone());
                    if utils::is_expired(&provider.config["expiry"]) {
                        let client = oauth2::CredentialsClient::new()?;
                        let token = client
                            .request_token(&["https://www.googleapis.com/auth/cloud-platform".to_string()])
                            .await?;
                        self.token = Some(token.access_token);
                    }
                }
                if let Some(id_token) = provider.config.get("id-token") {
                    self.token = Some(id_token.clone());
                }

                if self.token.is_none() {
                    if let Some(cmd) = provider.config.get("cmd-path") {
                        let params = provider.config.get("cmd-args").cloned().unwrap_or_default();

                        let output = std::process::Command::new(cmd)
                            .args(params.trim().split(' '))
                            .output()
                            .map_err(|e| {
                                ConfigError::AuthExec(format!("Executing {:} failed: {:?}", cmd, e))
                            })?;

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
                                        self.token = Some(res.clone());
                                    } else {
                                        return Err(ConfigError::AuthExec(format!(
                                            "Target value at {:} is not a string",
                                            pure_path
                                        ))
                                        .into());
                                    }
                                }
                                Err(e) => {
                                    return Err(ConfigError::AuthExec(format!(
                                        "Could not extract JSON value: {:}",
                                        e
                                    ))
                                    .into());
                                }
                                _ => {
                                    return Err(ConfigError::AuthExec(format!(
                                        "Target value {:} not found",
                                        pure_path
                                    ))
                                    .into());
                                }
                            };
                        } else {
                            self.token = Some(
                                std::str::from_utf8(&output.stdout)
                                    .map_err(|e| {
                                        ConfigError::AuthExec(format!("Result is not a string {:?} ", e))
                                    })?
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
            }
            None => {}
        };
        Ok(())
    }

    pub(crate) fn load_client_certificate(&self) -> Result<Vec<u8>> {
        utils::data_or_file_with_base64(&self.client_certificate_data, &self.client_certificate)
    }

    pub(crate) fn load_client_key(&self) -> Result<Vec<u8>> {
        utils::data_or_file_with_base64(&self.client_key_data, &self.client_key)
    }
}

#[cfg(test)]
mod test {
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
        auth_info.load_gcp().await?;
        assert_eq!(auth_info.token, Some("my_token".to_owned()));

        Ok(())
    }
}
