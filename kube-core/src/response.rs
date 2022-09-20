//! Generic api response types
use serde::Deserialize;

/// A Kubernetes status object
///
/// Equivalent to Status in k8s-openapi except we have have simplified options
#[derive(Deserialize, Debug)]
pub struct Status {
    /// Suggested HTTP return code (0 if unset)
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub code: u16,

    /// Status of the operation
    ///
    /// One of: `Success` or `Failure` - [more info](https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#spec-and-status)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,

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

/// Status details object on the [`Status`] object
#[derive(Deserialize, Debug)]
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
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub retry_after_seconds: u32,
}

/// Status cause object on the [`StatusDetails`] object
#[derive(Deserialize, Debug)]
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
