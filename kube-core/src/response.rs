//! Generic api response types
use serde::{Deserialize, Serialize};

/// A Kubernetes status object
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Eq)]
pub struct Status {
    /// Status of the operation
    ///
    /// One of: `Success` or `Failure` - [more info](https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StatusSummary>,

    /// Suggested HTTP return code (0 if unset)
    #[serde(default, skip_serializing_if = "is_u16_zero")]
    pub code: u16,

    /// A human-readable  description of the status of this operation
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,

    /// A machine-readable description of why this operation is in the “Failure” status.
    ///
    /// If this value is empty there is no information available.
    /// A Reason clarifies an HTTP status code but does not override it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,

    /// Extended data associated with the reason.
    ///
    /// Each reason may define its own extended details.
    /// This field is optional and the data returned is not guaranteed to conform to any schema except that defined by the reason type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,
}

impl Status {
    /// Returns a successful `Status`
    pub fn success() -> Self {
        Status {
            status: Some(StatusSummary::Success),
            code: 0,
            message: String::new(),
            reason: String::new(),
            details: None,
        }
    }

    /// Returns an unsuccessful `Status`
    pub fn failure(message: &str, reason: &str) -> Self {
        Status {
            status: Some(StatusSummary::Failure),
            code: 0,
            message: message.to_string(),
            reason: reason.to_string(),
            details: None,
        }
    }

    /// Sets an explicit HTTP status code
    pub fn with_code(mut self, code: u16) -> Self {
        self.code = code;
        self
    }

    /// Adds details to the `Status`
    pub fn with_details(mut self, details: StatusDetails) -> Self {
        self.details = Some(details);
        self
    }

    /// Checks if this `Status` represents success
    ///
    /// Note that it is possible for `Status` to be in indeterminate state
    /// when both `is_success` and `is_failure` return false.
    pub fn is_success(&self) -> bool {
        self.status == Some(StatusSummary::Success)
    }

    /// Checks if this `Status` represents failure
    ///
    /// Note that it is possible for `Status` to be in indeterminate state
    /// when both `is_success` and `is_failure` return false.
    pub fn is_failure(&self) -> bool {
        self.status == Some(StatusSummary::Failure)
    }
}

/// Overall status of the operation - whether it succeeded or not
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy)]
pub enum StatusSummary {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
}

/// Status details object on the [`Status`] object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StatusDetails {
    /// The name attribute of the resource associated with the status StatusReason (when there is a single name which can be described)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,

    /// The group attribute of the resource associated with the status StatusReason
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,

    /// The kind attribute of the resource associated with the status StatusReason
    ///
    /// On some operations may differ from the requested resource Kind - [more info](https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,

    /// UID of the resource (when there is a single resource which can be described)
    ///
    /// [More info](http://kubernetes.io/docs/user-guide/identifiers#uids)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub uid: String,

    /// The Causes vector includes more details associated with the failure
    ///
    /// Not all StatusReasons may provide detailed causes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<StatusCause>,

    /// If specified, the time in seconds before the operation should be retried.
    ///
    /// Some errors may indicate the client must take an alternate action -
    /// for those errors this field may indicate how long to wait before taking the alternate action.
    #[serde(default, skip_serializing_if = "is_u32_zero")]
    pub retry_after_seconds: u32,
}

/// Status cause object on the [`StatusDetails`] object
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct StatusCause {
    /// A machine-readable description of the cause of the error. If this value is empty there is no information available.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,

    /// A human-readable description of the cause of the error. This field may be presented as-is to a reader.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,

    /// The field of the resource that has caused this error, as named by its JSON serialization
    ///
    /// May include dot and postfix notation for nested attributes. Arrays are zero-indexed.
    /// Fields may appear more than once in an array of causes due to fields having multiple errors.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
}

fn is_u16_zero(&v: &u16) -> bool {
    v == 0
}

fn is_u32_zero(&v: &u32) -> bool {
    v == 0
}

#[cfg(test)]
mod test {
    use super::Status;

    // ensure our status schema is sensible
    #[test]
    fn delete_deserialize_test() {
        let statusresp = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"name":"some-app","group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s: Status = serde_json::from_str::<Status>(statusresp).unwrap();
        assert_eq!(s.details.unwrap().name, "some-app");

        let statusnoname = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s2: Status = serde_json::from_str::<Status>(statusnoname).unwrap();
        assert_eq!(s2.details.unwrap().name, ""); // optional probably better..
    }
}
