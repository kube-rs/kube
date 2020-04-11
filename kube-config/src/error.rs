//! Error handling in the [`kube-config`][crate]

use thiserror::Error;

/// Possible errors when working with [`kube-config`][crate]
#[derive(Error, Debug)]
pub enum Error {
    /// Reqwest error
    #[error("ReqwestError: {0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Configuration error
    #[error("Error loading kubeconfig: {0}")]
    Kubeconfig(String),

    /// An error with configuring SSL occured
    #[error("SslError: {0}")]
    SslError(String),

    /// An error from openssl when handling configuration
    #[cfg(feature = "native-tls")]
    #[error("OpensslError: {0}")]
    OpensslError(#[from] openssl::error::ErrorStack),
}
