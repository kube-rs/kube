//! Error handling in [`kube`][crate]
use std::path::PathBuf;

use thiserror::Error;

pub use kube_core::ErrorResponse;

/// Possible errors when working with [`kube`][crate]
#[cfg_attr(docsrs, doc(cfg(any(feature = "config", feature = "client"))))]
#[derive(Error, Debug)]
pub enum Error {
    /// ApiError for when things fail
    ///
    /// This can be parsed into as an error handling fallback.
    /// It's also used in `WatchEvent` from watch calls.
    ///
    /// It's quite common to get a `410 Gone` when the `resourceVersion` is too old.
    #[error("ApiError: {0} ({0:?})")]
    Api(#[source] ErrorResponse),

    /// Hyper error
    #[cfg(feature = "client")]
    #[error("HyperError: {0}")]
    HyperError(#[source] hyper::Error),
    /// Service error
    #[cfg(feature = "client")]
    #[error("ServiceError: {0}")]
    Service(#[source] tower::BoxError),

    /// UTF-8 Error
    #[error("UTF-8 Error: {0}")]
    FromUtf8(#[source] std::string::FromUtf8Error),

    /// Returned when failed to find a newline character within max length.
    /// Only returned by `Client::request_events` and this should never happen as
    /// the max is `usize::MAX`.
    #[error("Error finding newline character")]
    LinesCodecMaxLineLengthExceeded,

    /// Returned on `std::io::Error` when reading event stream.
    #[error("Error reading events stream: {0}")]
    ReadEvents(#[source] std::io::Error),

    /// Http based error
    #[error("HttpError: {0}")]
    HttpError(#[source] http::Error),

    /// Failed to construct a URI.
    #[error("InvalidUri: {0}")]
    InvalidUri(#[source] http::uri::InvalidUri),

    /// Common error case when requesting parsing into own structs
    #[error("Error deserializing response")]
    SerdeError(#[source] serde_json::Error),

    /// Failed to build request
    #[error("Failed to build request: {0}")]
    BuildRequest(#[source] kube_core::request::Error),

    /// Configuration error
    #[error("Error loading kubeconfig: {0}")]
    Kubeconfig(#[source] ConfigError),

    /// Discovery errors
    #[error("Error from discovery: {0}")]
    Discovery(#[source] DiscoveryError),

    /// An error with configuring SSL occured
    #[error("SslError: {0}")]
    SslError(String),

    /// An error from openssl when handling configuration
    #[cfg(feature = "native-tls")]
    #[cfg_attr(docsrs, doc(cfg(feature = "native-tls")))]
    #[error("OpensslError: {0}")]
    OpensslError(#[source] openssl::error::ErrorStack),

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

    /// Errors related to client auth
    #[cfg(feature = "client")]
    #[cfg_attr(docsrs, doc(cfg(feature = "client")))]
    #[error("auth error: {0}")]
    Auth(#[source] crate::client::AuthError),
}

#[derive(Error, Debug)]
// Redundant with the error messages and machine names
#[allow(missing_docs)]
/// Possible errors when loading config
pub enum ConfigError {
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

    #[error("Unable to load incluster default namespace: {0}")]
    InvalidInClusterNamespace(#[source] Box<Error>),

    #[error("Unable to load in cluster token: {0}")]
    InvalidInClusterToken(#[source] Box<Error>),

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

    #[error("Failed to parse Kubeconfig YAML: {0}")]
    ParseYaml(#[source] serde_yaml::Error),

    #[error("Failed to find a single YAML document in Kubeconfig: {0}")]
    EmptyKubeconfig(PathBuf),
}

#[derive(Error, Debug)]
// Redundant with the error messages and machine names
#[allow(missing_docs)]
/// Possible errors when using API discovery
pub enum DiscoveryError {
    #[error("Invalid GroupVersion: {0}")]
    InvalidGroupVersion(String),
    #[error("Missing Kind: {0}")]
    MissingKind(String),
    #[error("Missing Api Group: {0}")]
    MissingApiGroup(String),
    #[error("Missing MissingResource: {0}")]
    MissingResource(String),
    #[error("Empty Api Group: {0}")]
    EmptyApiGroup(String),
}
