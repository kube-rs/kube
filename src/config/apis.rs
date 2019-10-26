use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use failure::ResultExt;
use crate::{Result, ErrorKind};
use crate::config::utils;
use crate::oauth2;

/// Config stores information to connect remote kubernetes cluster.
///
/// This type (and its children) are exposed for convenience only.
/// Please load a `Configuration` object for use with a `kube::Client`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
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

/// Cluster stores information to connect kubernetes cluster.
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

impl Config {
    pub(crate) fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
        let f = File::open(path)
            .context(ErrorKind::KubeConfig("Unable to open config file".into()))?;
        let config = serde_yaml::from_reader(f)
            .context(ErrorKind::KubeConfig("Unable to parse config file as yaml".into()))?;
        Ok(config)
    }
}

impl Cluster {
    pub(crate) fn load_certificate_authority(&self) -> Result<Vec<u8>> {
        let res = utils::data_or_file_with_base64(
            &self.certificate_authority_data,
            &self.certificate_authority,
        ).context(ErrorKind::KubeConfig("Unable to decode base64 certificates".into()))?;
        Ok(res)
    }
}

impl AuthInfo {
    pub(crate) async fn load_gcp(&mut self) -> Result<bool> {
        match &self.auth_provider {
            Some(provider) => {
                if let Some(access_token) = provider.config.get("access-token") {
                    self.token = Some(access_token.clone());
                    if utils::is_expired(&provider.config["expiry"]) {
                        let client = oauth2::CredentialsClient::new()?;
                        let token = client.request_token(&vec![
                            "https://www.googleapis.com/auth/cloud-platform".to_string(),
                        ]).await?;
                        self.token = Some(token.access_token);
                    }
                }
                if let Some(id_token) = provider.config.get("id-token") {
                    self.token = Some(id_token.clone());
                }
            }
            None => {}
        };
        Ok(true)
    }

    pub(crate) fn load_client_certificate(&self) -> Result<Vec<u8>> {
        Ok(utils::data_or_file_with_base64(&self.client_certificate_data, &self.client_certificate)
            .context(ErrorKind::KubeConfig("Unable to decode base64 client cert".into()))?)
    }

    pub(crate) fn load_client_key(&self) -> Result<Vec<u8>> {
        Ok(utils::data_or_file_with_base64(&self.client_key_data, &self.client_key)
            .context(ErrorKind::KubeConfig("Unable to decode base64 client key".into()))?)
    }
}
