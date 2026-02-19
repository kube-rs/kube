//! Generic api response types
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A Kubernetes status object
///
/// This struct is returned by the Kubernetes API on failures,
/// and bubbles up to users inside a [`kube::Error::Api`] variant
/// when client requests fail in [`kube::Client`].
///
/// To match on specific error cases, you can;
///
/// ```no_compile
/// match err {
///     kube::Error::Api(s) if s.is_not_found() => {...},
/// }
/// ```
///
/// or in a standalone `if` statement with [std::matches];
///
/// ```no_compile
/// if std::matches!(err, kube::Error::Api(s) if s.is_forbidden()) {...}
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq, Error)]
#[error("{message}: {reason}")]
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

    /// Standard list metadata - [more info](https://git.k8s.io/community/contributors/devel/sig-architecture/api-conventions.md#types-kinds)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::ListMeta>,

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
    /// Returns a boxed `Status`
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    /// Returns a successful `Status`
    pub fn success() -> Self {
        Status {
            status: Some(StatusSummary::Success),
            code: 0,
            message: String::new(),
            metadata: None,
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
            metadata: None,
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

    /// Checks if this `Status` represents not found error
    ///
    /// Note that it is possible for `Status` to be in indeterminate state
    /// when both `is_success` and `is_failure` return false.
    pub fn is_not_found(&self) -> bool {
        self.reason_or_code(reason::NOT_FOUND, 404)
    }

    /// Checks if this `Status` indicates that a specified resource already exists.
    pub fn is_already_exists(&self) -> bool {
        self.reason == reason::ALREADY_EXISTS
    }

    /// Checks if this `Status` indicates update conflict
    pub fn is_conflict(&self) -> bool {
        self.reason_or_code(reason::CONFLICT, 409)
    }

    /// Checks if this `Status` indicates that the request is forbidden and cannot
    /// be completed as requested.
    pub fn is_forbidden(&self) -> bool {
        self.reason_or_code(reason::FORBIDDEN, 403)
    }

    /// Checks if this `Status` indicates that provided resource is not valid.
    pub fn is_invalid(&self) -> bool {
        self.reason_or_code(reason::INVALID, 422)
    }

    // This helper function is used by other is_xxx helpers.
    // Its implementation follows that of the Go client.
    // See for example
    // https://github.com/kubernetes/apimachinery/blob/v0.35.0/pkg/api/errors/errors.go#L529
    fn reason_or_code(&self, reason: &str, code: u16) -> bool {
        self.reason == reason || (!reason::is_known(reason) && self.code == code)
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

/// StatusReason is an enumeration of possible failure causes.  Each StatusReason
/// must map to a single HTTP status code, but multiple reasons may map
/// to the same HTTP status code.
///
/// See https://pkg.go.dev/k8s.io/apimachinery/pkg/apis/meta/v1#StatusReason
/// for the authoritative list of reasons in Go universe.
pub mod reason {

    /// StatusReasonUnknown means the server has declined to indicate a specific reason.
    /// The details field may contain other information about this error.
    /// Status code 500.
    pub const UNKNOWN: &str = "";

    /// StatusReasonUnauthorized means the server can be reached and understood the request, but requires
    /// the user to present appropriate authorization credentials (identified by the WWW-Authenticate header)
    /// in order for the action to be completed. If the user has specified credentials on the request, the
    /// server considers them insufficient.
    /// Status code 401
    pub const UNAUTHORIZED: &str = "Unauthorized";

    /// StatusReasonForbidden means the server can be reached and understood the request, but refuses
    /// to take any further action.  It is the result of the server being configured to deny access for some reason
    /// to the requested resource by the client.
    /// Details (optional):
    ///   "kind" string - the kind attribute of the forbidden resource
    ///                   on some operations may differ from the requested
    ///                   resource.
    ///   "id"   string - the identifier of the forbidden resource
    /// Status code 403
    pub const FORBIDDEN: &str = "Forbidden";

    /// StatusReasonNotFound means one or more resources required for this operation
    /// could not be found.
    /// Details (optional):
    ///   "kind" string - the kind attribute of the missing resource
    ///                   on some operations may differ from the requested
    ///                   resource.
    ///   "id"   string - the identifier of the missing resource
    /// Status code 404
    pub const NOT_FOUND: &str = "NotFound";

    /// StatusReasonAlreadyExists means the resource you are creating already exists.
    /// Details (optional):
    ///   "kind" string - the kind attribute of the conflicting resource
    ///   "id"   string - the identifier of the conflicting resource
    /// Status code 409
    pub const ALREADY_EXISTS: &str = "AlreadyExists";

    /// StatusReasonConflict means the requested operation cannot be completed
    /// due to a conflict in the operation. The client may need to alter the
    /// request. Each resource may define custom details that indicate the
    /// nature of the conflict.
    /// Status code 409
    pub const CONFLICT: &str = "Conflict";

    /// StatusReasonGone means the item is no longer available at the server and no
    /// forwarding address is known.
    /// Status code 410
    pub const GONE: &str = "Gone";

    /// StatusReasonInvalid means the requested create or update operation cannot be
    /// completed due to invalid data provided as part of the request. The client may
    /// need to alter the request. When set, the client may use the StatusDetails
    /// message field as a summary of the issues encountered.
    /// Details (optional):
    ///   "kind" string - the kind attribute of the invalid resource
    ///   "id"   string - the identifier of the invalid resource
    ///   "causes"      - one or more StatusCause entries indicating the data in the
    ///                   provided resource that was invalid.  The code, message, and
    ///                   field attributes will be set.
    /// Status code 422
    pub const INVALID: &str = "Invalid";

    /// StatusReasonServerTimeout means the server can be reached and understood the request,
    /// but cannot complete the action in a reasonable time. The client should retry the request.
    /// This is may be due to temporary server load or a transient communication issue with
    /// another server. Status code 500 is used because the HTTP spec provides no suitable
    /// server-requested client retry and the 5xx class represents actionable errors.
    /// Details (optional):
    ///   "kind" string - the kind attribute of the resource being acted on.
    ///   "id"   string - the operation that is being attempted.
    ///   "retryAfterSeconds" int32 - the number of seconds before the operation should be retried
    /// Status code 500
    pub const SERVER_TIMEOUT: &str = "ServerTimeout";

    /// StatusReasonStoreReadError means that the server encountered an error while
    /// retrieving resources from the backend object store.
    /// This may be due to backend database error, or because processing of the read
    /// resource failed.
    /// Details:
    ///   "kind" string - the kind attribute of the resource being acted on.
    ///   "name" string - the prefix where the reading error(s) occurred
    ///   "causes" []StatusCause
    ///      - (optional):
    ///        - "type" CauseType - CauseTypeUnexpectedServerResponse
    ///        - "message" string - the error message from the store backend
    ///        - "field" string - the full path with the key of the resource that failed reading
    ///
    /// Status code 500
    pub const STORAGE_READ_ERROR: &str = "StorageReadError";

    /// StatusReasonTimeout means that the request could not be completed within the given time.
    /// Clients can get this response only when they specified a timeout param in the request,
    /// or if the server cannot complete the operation within a reasonable amount of time.
    /// The request might succeed with an increased value of timeout param. The client *should*
    /// wait at least the number of seconds specified by the retryAfterSeconds field.
    /// Details (optional):
    ///   "retryAfterSeconds" int32 - the number of seconds before the operation should be retried
    /// Status code 504
    pub const TIMEOUT: &str = "Timeout";

    /// StatusReasonTooManyRequests means the server experienced too many requests within a
    /// given window and that the client must wait to perform the action again. A client may
    /// always retry the request that led to this error, although the client should wait at least
    /// the number of seconds specified by the retryAfterSeconds field.
    /// Details (optional):
    ///   "retryAfterSeconds" int32 - the number of seconds before the operation should be retried
    /// Status code 429
    pub const TOO_MANY_REQUESTS: &str = "TooManyRequests";

    /// StatusReasonBadRequest means that the request itself was invalid, because the request
    /// doesn't make any sense, for example deleting a read-only object.  This is different than
    /// StatusReasonInvalid above which indicates that the API call could possibly succeed, but the
    /// data was invalid.  API calls that return BadRequest can never succeed.
    /// Status code 400
    pub const BAD_REQUEST: &str = "BadRequest";

    /// StatusReasonMethodNotAllowed means that the action the client attempted to perform on the
    /// resource was not supported by the code - for instance, attempting to delete a resource that
    /// can only be created. API calls that return MethodNotAllowed can never succeed.
    /// Status code 405
    pub const METHOD_NOT_ALLOWED: &str = "MethodNotAllowed";

    /// StatusReasonNotAcceptable means that the accept types indicated by the client were not acceptable
    /// to the server - for instance, attempting to receive protobuf for a resource that supports only json and yaml.
    /// API calls that return NotAcceptable can never succeed.
    /// Status code 406
    pub const NOT_ACCEPTABLE: &str = "NotAcceptable";

    /// StatusReasonRequestEntityTooLarge means that the request entity is too large.
    /// Status code 413
    pub const REQUEST_ENTITY_TOO_LARGE: &str = "RequestEntityTooLarge";

    /// StatusReasonUnsupportedMediaType means that the content type sent by the client is not acceptable
    /// to the server - for instance, attempting to send protobuf for a resource that supports only json and yaml.
    /// API calls that return UnsupportedMediaType can never succeed.
    /// Status code 415
    pub const UNSUPPORTED_MEDIA_TYPE: &str = "UnsupportedMediaType";

    /// StatusReasonInternalError indicates that an internal error occurred, it is unexpected
    /// and the outcome of the call is unknown.
    /// Details (optional):
    ///   "causes" - The original error
    /// Status code 500
    pub const INTERNAL_ERROR: &str = "InternalError";

    /// StatusReasonExpired indicates that the request is invalid because the content you are requesting
    /// has expired and is no longer available. It is typically associated with watches that can't be
    /// serviced.
    /// Status code 410 (gone)
    pub const EXPIRED: &str = "Expired";

    /// StatusReasonServiceUnavailable means that the request itself was valid,
    /// but the requested service is unavailable at this time.
    /// Retrying the request after some time might succeed.
    /// Status code 503
    pub const SERVICE_UNAVAILABLE: &str = "ServiceUnavailable";

    /// Checks status reason to be one of the known reasons.
    pub fn is_known(reason: &str) -> bool {
        KNOWN_REASONS.contains(&reason)
    }

    const KNOWN_REASONS: &[&str] = &[
        UNAUTHORIZED,
        FORBIDDEN,
        NOT_FOUND,
        ALREADY_EXISTS,
        CONFLICT,
        GONE,
        INVALID,
        SERVER_TIMEOUT,
        STORAGE_READ_ERROR,
        TIMEOUT,
        TOO_MANY_REQUESTS,
        BAD_REQUEST,
        METHOD_NOT_ALLOWED,
        NOT_ACCEPTABLE,
        REQUEST_ENTITY_TOO_LARGE,
        UNSUPPORTED_MEDIA_TYPE,
        INTERNAL_ERROR,
        EXPIRED,
        SERVICE_UNAVAILABLE,
    ];
}

#[cfg(test)]
mod test {

    use super::*;

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

    #[test]
    fn feature_with_details1() {
        let status = r#"
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
            }"#;
        let s = serde_json::from_str::<Status>(status).unwrap();
        assert!(s.is_invalid());
        assert_eq!(s.status.unwrap(), StatusSummary::Failure);
        assert_eq!(s.details.unwrap().name, "test");
    }

    #[test]
    fn failure_with_details2() {
        let status = r#"
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
            }"#;
        let s = serde_json::from_str::<Status>(status).unwrap();
        assert!(s.is_invalid());
        assert_eq!(s.status.unwrap(), StatusSummary::Failure);
        assert_eq!(s.details.unwrap().name, "test_");
    }

    #[test]
    fn failure_with_details3() {
        let status1 = r#"
            {
                "kind": "Status",
                "apiVersion": "v1",
                "metadata": {},
                "status": "Failure",
                "message": "pods \"foobar-1\" not found",
                "reason": "NotFound",
                "details": {
                    "name": "foobar-1",
                    "kind": "pods"
                },
                "code": 404
            }"#;
        let s = serde_json::from_str::<Status>(status1).unwrap();
        assert!(s.is_not_found());
        assert_eq!(s.status.unwrap(), StatusSummary::Failure);
        assert_eq!(s.details.unwrap().name, "foobar-1");
    }

    #[test]
    fn expired_with_continue_token() {
        let status = r#"
            {
              "kind": "Status",
              "apiVersion": "v1",
              "metadata": {
                "continue": "<NEW_CONTINUE_TOKEN>"
              },
              "status": "Failure",
              "message": "The provided continue parameter is too old to display a consistent list result.",
              "reason": "Expired",
              "code": 410
            }"#;
        let s = serde_json::from_str::<Status>(status).unwrap();
        assert_eq!(s.reason, "Expired");
        assert_eq!(s.code, 410);
        assert_eq!(
            s.metadata.unwrap().continue_.as_deref(),
            Some("<NEW_CONTINUE_TOKEN>")
        );
    }
}
