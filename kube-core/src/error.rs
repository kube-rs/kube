use serde::{Deserialize, Serialize};
use thiserror::Error;

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
