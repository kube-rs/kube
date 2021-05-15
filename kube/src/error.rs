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
    /// It's also used in `WatchEvent` from watch calls.
    ///
    /// It's quite common to get a `410 Gone` when the `resourceVersion` is too old.
    #[error("ApiError")]
    Api(#[source] ErrorResponse),

    /// ConnectionError for when TcpStream fails to connect.
    #[error("ConnectionError")]
    Connection(#[source] std::io::Error),

    /// Hyper error
    #[error("HyperError")]
    HyperError(#[from] hyper::Error),

    /// Service error
    #[error("ServiceError")]
    Service(#[source] tower::BoxError),

    /// UTF-8 Error
    #[error("UTF-8 Error")]
    FromUtf8(#[from] std::string::FromUtf8Error),

    /// Returned when failed to find a newline character within max length.
    /// Only returned by `Client::request_events` and this should never happen as
    /// the max is `usize::MAX`.
    #[error("Error finding newline character")]
    LinesCodecMaxLineLengthExceeded,

    /// Returned on `std::io::Error` when reading event stream.
    #[error("Error reading events stream")]
    ReadEvents(#[source] std::io::Error),

    /// Http based error
    #[error("HttpError")]
    HttpError(#[from] http::Error),

    /// Url conversion error
    #[error("InternalUrlError")]
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

    /// A request validation failed
    #[error("Request validation failed with {0}")]
    RequestValidation(String),

    /// A dynamic type conversion failure
    #[error("Dynamic type conversion failed {0}")]
    DynamicType(String),

    /// Configuration error
    #[error("Error loading kubeconfig")]
    Kubeconfig(#[from] ConfigError),

    /// An error with configuring SSL occured
    #[error("SslError: {0}")]
    SslError(String),

    /// An error from openssl when handling configuration
    #[cfg(feature = "native-tls")]
    #[error("OpensslError")]
    OpensslError(#[from] openssl::error::ErrorStack),

    /// The server did not respond with [`SWITCHING_PROTOCOLS`] status when upgrading the
    /// connection.
    ///
    /// [`SWITCHING_PROTOCOLS`]: http::status::StatusCode::SWITCHING_PROTOCOLS
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    #[error("Failed to switch protocol. Status code: {0}")]
    ProtocolSwitch(http::status::StatusCode),

    /// `Upgrade` header was not set to `websocket` (case insensitive)
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    #[error("Upgrade header was not set to websocket")]
    MissingUpgradeWebSocketHeader,

    /// `Connection` header was not set to `Upgrade` (case insensitive)
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    #[error("Connection header was not set to Upgrade")]
    MissingConnectionUpgradeHeader,

    /// `Sec-WebSocket-Accept` key mismatched.
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    #[error("Sec-WebSocket-Accept key mismatched")]
    SecWebSocketAcceptKeyMismatch,

    /// `Sec-WebSocket-Protocol` mismatched.
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    #[error("Sec-WebSocket-Protocol mismatched")]
    SecWebSocketProtocolMismatch,
}

#[derive(Error, Debug)]
// Redundant with the error messages and machine names
#[allow(missing_docs)]
/// Possible errors when loading config
pub enum ConfigError {
    #[error("Invalid basic auth")]
    InvalidBasicAuth(#[source] InvalidHeaderValue),

    #[error("Invalid bearer token")]
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

    #[error("Failed to determine current context")]
    CurrentContextNotSet,

    #[error("Merging kubeconfig with mismatching kind")]
    KindMismatch,

    #[error("Merging kubeconfig with mismatching apiVersion")]
    ApiVersionMismatch,

    #[error("Unable to load in cluster config, {hostenv} and {portenv} must be defined")]
    /// One or more required in-cluster config options are missing
    MissingInClusterVariables {
        hostenv: &'static str,
        portenv: &'static str,
    },

    #[error("Unable to load incluster default namespace")]
    InvalidInClusterNamespace(#[source] Box<Error>),

    #[error("Unable to load in cluster token")]
    InvalidInClusterToken(#[source] Box<Error>),

    #[error("Malformed url")]
    MalformedUrl(#[from] url::ParseError),

    #[error("exec-plugin response did not contain a status")]
    ExecPluginFailed,

    #[error("Malformed token expiration date")]
    MalformedTokenExpirationDate(#[source] chrono::ParseError),

    #[cfg(feature = "oauth")]
    #[cfg_attr(docsrs, doc(cfg(feature = "oauth")))]
    #[error("OAuth Error")]
    OAuth(#[from] OAuthError),

    #[error("Unable to load config file")]
    LoadConfigFile(#[source] Box<Error>),

    #[error("Unable to load current context: {context_name}")]
    LoadContext { context_name: String },

    #[error("Unable to load cluster of context: {cluster_name}")]
    LoadClusterOfContext { cluster_name: String },

    #[error("Unable to find named user: {user_name}")]
    FindUser { user_name: String },

    #[error("Unable to find path of kubeconfig")]
    NoKubeconfigPath,

    #[error("Failed to decode base64")]
    Base64Decode(#[source] base64::DecodeError),

    #[error("Failed to compute the absolute path of '{path:?}'")]
    NoAbsolutePath { path: PathBuf },

    #[error("Failed to read '{path:?}'")]
    ReadFile {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to get data/file with base64 format")]
    NoBase64FileOrData,

    #[error("Failed to get data/file")]
    NoFileOrData,

    #[error("Failed to parse Kubeconfig YAML")]
    ParseYaml(#[source] serde_yaml::Error),

    #[error("Failed to find a single YAML document in Kubeconfig: {0}")]
    EmptyKubeconfig(PathBuf),

    #[error("Unable to run auth exec")]
    AuthExecStart(#[source] std::io::Error),

    #[error("Auth exec command '{cmd}' failed with status {status}: {out:?}")]
    AuthExecRun {
        cmd: String,
        status: std::process::ExitStatus,
        out: std::process::Output,
    },

    #[error("Failed to parse auth exec output")]
    AuthExecParse(#[source] serde_json::Error),

    #[error("Failed exec auth: {0}")]
    AuthExec(String),
}

#[cfg(feature = "oauth")]
#[cfg_attr(docsrs, doc(cfg(feature = "oauth")))]
#[derive(Error, Debug)]
// Redundant with the error messages and machine names
#[allow(missing_docs)]
/// Possible errors when requesting token with OAuth
pub enum OAuthError {
    /// Missing GOOGLE_APPLICATION_CREDENTIALS env
    #[error("Missing GOOGLE_APPLICATION_CREDENTIALS env")]
    MissingGoogleCredentials,

    #[error("Unable to load OAuth credentials file")]
    LoadCredentials(#[source] std::io::Error),

    #[error("Unable to parse OAuth credentials file")]
    ParseCredentials(#[source] serde_json::Error),

    #[error("Credentials file had invalid key format")]
    InvalidKeyFormat(#[source] tame_oauth::Error),

    #[error("Credentials file had invalid RSA key")]
    InvalidRsaKey(#[source] tame_oauth::Error),

    #[error("Unable to request token")]
    RequestToken(#[source] hyper::Error),

    #[error("Fail to retrieve new credential {0:?}")]
    RetrieveCredentials(#[source] tame_oauth::Error),

    #[error("Unable to parse token")]
    ParseToken(#[source] serde_json::Error),

    #[error("Unknown OAuth error: {0}")]
    Unknown(String),
}

#[cfg(feature = "oauth")]
#[cfg_attr(docsrs, doc(cfg(feature = "oauth")))]
impl From<OAuthError> for Error {
    fn from(e: OAuthError) -> Self {
        ConfigError::OAuth(e).into()
    }
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
