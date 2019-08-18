use std::process::Command;

use failure::ResultExt;
use crate::{Error, Result, ErrorKind};
use crate::config::apis;

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

pub fn auth_exec(auth: &apis::ExecConfig) -> Result<ExecCredential> {
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
    let out = cmd.output()
        .context(ErrorKind::KubeConfig("Unable to run auth exec".into()))?;
    if !out.status.success() {
        let err = format!("command `{:?}` failed: {:?}", cmd, out);
        return Err(Error::from(ErrorKind::KubeConfig(err)));
    }
    let creds = serde_json::from_slice(&out.stdout)
        .context(ErrorKind::KubeConfig("Unable to parse auth exec result as json".into()))?;

    Ok(creds)
}
