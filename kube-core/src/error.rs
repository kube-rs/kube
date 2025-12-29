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
    /// Extended data associated with the reason.
    /// Each reason may define its own extended details.
    pub details: Option<StatusDetails>,
}

/// StatusDetails is a set of additional properties that MAY be set by the server
/// to provide additional information about a response.
/// The Reason field of a Status object defines what attributes will be set.
/// Clients must ignore fields that do not match the defined type of each attribute,
/// and should assume that any attribute may be empty, invalid, or under defined.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusDetails {
    /// The Causes array includes more details associated with the StatusReason failure.
    /// Not all StatusReasons may provide detailed causes.
    pub causes: Option<Vec<StatusCause>>,

    /// The group attribute of the resource associated with the status StatusReason.
    pub group: Option<String>,

    /// The kind attribute of the resource associated with the status StatusReason.
    /// On some operations may differ from the requested resource Kind.
    /// More info: https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds
    pub kind: Option<String>,

    /// The name attribute of the resource associated with the status StatusReason
    /// (when there is a single name which can be described).
    pub name: Option<String>,

    /// If specified, the time in seconds before the operation should be retried.
    /// Some errors may indicate the client must take an alternate action - for
    /// those errors this field may indicate how long to wait before taking the
    /// alternate action.
    pub retry_after_seconds: Option<i32>,

    /// UID of the resource. (when there is a single resource which can be described).
    /// More info: https://kubernetes.io/docs/concepts/overview/working-with-objects/names#uids
    pub uid: Option<String>,
}

/// StatusCause provides more information about an api.Status failure,
/// including cases when multiple errors are encountered.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusCause {
    /// The field of the resource that has caused this error, as named by its JSON serialization.
    /// May include dot and postfix notation for nested attributes. Arrays are zero-indexed.
    /// Fields may appear more than once in an array of causes due to fields having multiple errors.
    /// Optional.
    ///
    /// Examples:
    ///   "name" - the field "name" on the current resource
    ///   "items\[0\].name" - the field "name" on the first array entry in "items"
    pub field: Option<String>,

    /// A human-readable description of the cause of the error.
    /// This field may be presented as-is to a reader.
    pub message: Option<String>,

    /// A machine-readable description of the cause of the error.
    /// If this value is empty there is no information available.
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATUS1: &str = r#"
    {
      "kind": "Status",
      "apiVersion": "v1",
      "metadata": {},
      "status": "Failure",
      "message": "leases.coordination.k8s.io \"test\" is invalid: metadata.resourceVersion: Invalid value: 0: must be specified for an update",
      "reason": "Invalid",
      "details": {
        "name": "test",
        "group": "coordination.k8s.io",
        "kind": "leases",
        "causes": [
          {
            "reason": "FieldValueInvalid",
            "message": "Invalid value: 0: must be specified for an update",
            "field": "metadata.resourceVersion"
          }
        ]
      },
      "code": 422
    }
    "#;

    const STATUS2: &str = r#"
    {
      "kind": "Status",
      "apiVersion": "v1",
      "metadata": {},
      "status": "Failure",
      "message": "Lease.coordination.k8s.io \"test_\" is invalid: metadata.name: Invalid value: \"test_\": a lowercase RFC 1123 subdomain must consist of lower case alphanumeric characters, '-' or '.', and must start and end with an alphanumeric character (e.g. 'example.com', regex used for validation is '[a-z0-9]([-a-z0-9]*[a-z0-9])?(\\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*')",
      "reason": "Invalid",
      "details": {
        "name": "test_",
        "group": "coordination.k8s.io",
        "kind": "Lease",
        "causes": [
          {
            "reason": "FieldValueInvalid",
            "message": "Invalid value: \"test_\": a lowercase RFC 1123 subdomain must consist of lower case alphanumeric characters, '-' or '.', and must start and end with an alphanumeric character (e.g. 'example.com', regex used for validation is '[a-z0-9]([-a-z0-9]*[a-z0-9])?(\\.[a-z0-9]([-a-z0-9]*[a-z0-9])?)*')",
            "field": "metadata.name"
          }
        ]
      },
      "code": 422
    }
    "#;

    fn error_response(text: &str) -> serde_json::Result<ErrorResponse> {
        serde_json::from_str(text)
    }

    #[test]
    fn status1() {
        let status1 = error_response(STATUS1).unwrap();
        assert_eq!(status1.code, 422);
        assert_eq!(status1.details.unwrap().name.unwrap(), "test");
    }

    #[test]
    fn status2() {
        let status2 = error_response(STATUS2).unwrap();
        assert_eq!(status2.code, 422);
        assert_eq!(status2.details.unwrap().name.unwrap(), "test_");
    }

    #[test]
    fn different() {
        let status1 = error_response(STATUS1).unwrap();
        let status2 = error_response(STATUS2).unwrap();
        assert_ne!(status1, status2);
    }
}
