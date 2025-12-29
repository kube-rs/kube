//! StatusReason is an enumeration of possible failure causes.  Each StatusReason
//! must map to a single HTTP status code, but multiple reasons may map
//! to the same HTTP status code.

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
