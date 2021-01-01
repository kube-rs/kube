//! Error handling in [`kube`][crate]

use http::header::InvalidHeaderValue;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Possible errors when working with [`kube`][crate]
#[derive(Error, Debug)]
pub enum Error {
    /// ApiError for when things fail
    ///
    /// This can be parsed into as an error handling fallback.
    /// Replacement data for [`reqwest::Response::error_for_status`],
    /// which is often lacking in good permission errors.
    /// It's also used in `WatchEvent` from watch calls.
    ///
    /// It's quite common to get a `410 Gone` when the `resourceVersion` is too old.
    #[error("ApiError: {0} ({0:?})")]
    Api(#[source] ErrorResponse),

    /// Reqwest error
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),
    /// Http based error
    #[error("HttpError: {0}")]
    HttpError(#[from] http::Error),

    /// Url conversion error
    #[error("InternalUrlError: {0}")]
    InternalUrlError(#[from] url::ParseError),

    /// Common error case when requesting parsing into own structs
    #[error("Error deserializing response")]
    SerdeError(#[from] serde_json::Error),

    /// Error building a request
    #[error("Error building request")]
    RequestBuild,

    /// Error sending a request
    #[error("Error executing request")]
    RequestSend,

    /// Error parsing a response
    #[error("Error parsing response")]
    RequestParse,

    /// An invalid method was used
    #[error("Invalid API method {0}")]
    InvalidMethod(String),

    /// A request validation failed
    #[error("Request validation failed with {0}")]
    RequestValidation(String),

    /// A dynamic resource conversion failure
    #[error("Dynamic resource conversion failed {0}")]
    DynamicResource(String),

    /// Configuration error
    #[error("Error loading kubeconfig: {0}")]
    Kubeconfig(#[from] ConfigError),

    /// An error with configuring SSL occured
    #[error("SslError: {0}")]
    SslError(String),

    /// An error from openssl when handling configuration
    #[cfg(feature = "native-tls")]
    #[error("OpensslError: {0}")]
    OpensslError(#[from] openssl::error::ErrorStack),
}

#[derive(Error, Debug)]
// Redundant with the error messages and machine names
#[allow(missing_docs)]
/// Possible errors when loading config
pub enum ConfigError {
    #[error("Invalid basic auth: {0}")]
    InvalidBasicAuth(#[source] InvalidHeaderValue),

    #[error("Invalid bearer token: {0}")]
    InvalidBearerToken(#[source] InvalidHeaderValue),

    #[error("Tried to refresh a token and got a non-refreshable token response")]
    /// Tried to refresh a token and got a non-refreshable token response
    UnrefreshableTokenResponse,

    #[error("Failed to infer config.. cluster env: ({cluster_env}), kubeconfig: ({kubeconfig})")]
    ConfigInferenceExhausted {
        cluster_env: Box<Error>,
        // We can only pick one source, but the kubeconfig failure is more likely to be a user error
        #[source]
        kubeconfig: Box<Error>,
    },

    #[error("Unable to load in cluster config, {hostenv} and {portenv} must be defined")]
    /// One or more required in-cluster config options are missing
    MissingInClusterVariables {
        hostenv: &'static str,
        portenv: &'static str,
    },

    #[error("Unable to load incluster default namespace: {0}")]
    InvalidInClusterNamespace(#[source] Box<Error>),

    #[error("Unable to load in cluster token: {0}")]
    InvalidInClusterToken(#[source] Box<Error>),

    #[error("Malformed url: {0}")]
    MalformedUrl(#[from] url::ParseError),

    #[error("exec-plugin response did not contain a status")]
    ExecPluginFailed,

    #[error("Malformed token expiration date: {0}")]
    MalformedTokenExpirationDate(#[source] chrono::ParseError),

    #[error("Missing GOOGLE_APPLICATION_CREDENTIALS env")]
    /// Missing GOOGLE_APPLICATION_CREDENTIALS env
    MissingGoogleCredentials,

    #[error("Unable to load OAuth2 credentials file: {0}")]
    OAuth2LoadCredentials(#[source] std::io::Error),
    #[error("Unable to parse OAuth2 credentials file: {0}")]
    OAuth2ParseCredentials(#[source] serde_json::Error),
    #[error("Unable to request token: {0}")]
    OAuth2RequestToken(#[source] reqwest::Error),
    #[error("Fail to retrieve new credential {0:?}")]
    OAuth2RetrieveCredentials(Box<reqwest::Response>),
    #[error("Unable to parse token: {0}")]
    OAuth2ParseToken(#[source] reqwest::Error),

    #[error("Unable to load config file: {0}")]
    LoadConfigFile(#[source] Box<Error>),
    #[error("Unable to load current context: {context_name}")]
    LoadContext { context_name: String },
    #[error("Unable to load cluster of context: {cluster_name}")]
    LoadClusterOfContext { cluster_name: String },
    #[error("Unable to find named user: {user_name}")]
    FindUser { user_name: String },

    #[error("Unable to find path of kubeconfig")]
    NoKubeconfigPath,

    #[error("Failed to decode base64: {0}")]
    Base64Decode(#[source] base64::DecodeError),
    #[error("Failed to compute the absolute path of '{path:?}'")]
    NoAbsolutePath { path: PathBuf },
    #[error("Failed to read '{path:?}': {source}")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to get data/file with base64 format")]
    NoBase64FileOrData,
    #[error("Failed to get data/file")]
    NoFileOrData,

    #[error("Failed to load certificate: {0}")]
    LoadCert(#[source] reqwest::Error),

    #[error("Failed to parse Kubeconfig YAML: {0}")]
    ParseYaml(#[source] serde_yaml::Error),

    #[error("Unable to run auth exec: {0}")]
    AuthExecStart(#[source] std::io::Error),
    #[error("Auth exec command '{cmd}' failed with status {status}: {out:?}")]
    AuthExecRun {
        cmd: String,
        status: std::process::ExitStatus,
        out: std::process::Output,
    },
    #[error("Failed to parse auth exec output: {0}")]
    AuthExecParse(#[source] serde_json::Error),
    #[error("Failed exec auth: {0}")]
    AuthExec(String),
}

/// An error response from the API.
#[derive(Error, Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
#[error("{message}: {reason}")]
pub struct ErrorResponse {
    /// The status
    pub status: String,
    /// A message about the error
    #[serde(default)]
    pub message: String,
    /// The reason for the error
    #[serde(default)]
    pub reason: String,
    /// The error code
    pub code: u16,
}
