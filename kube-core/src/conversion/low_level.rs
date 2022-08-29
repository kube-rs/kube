use crate::{Status, TypeMeta};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The `kind` field in [`TypeMeta`].
pub const META_KIND: &str = "ConversionReview";
/// The `api_version` field in [`TypeMeta`] on the v1 version.
pub const META_API_VERSION_V1: &str = "apiextensions.k8s.io/v1";

#[derive(Debug, Error)]
#[error("request missing in ConversionReview")]
/// Failed to convert `AdmissionReview` into `AdmissionRequest`.
pub struct ConvertConversionReviewError;

/// Struct that describes both request and response.
#[derive(Serialize, Deserialize)]
pub struct ConversionReview {
    /// Contains the API version and type of the request.
    #[serde(flatten)]
    pub types: TypeMeta,
    /// Contains conversion request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<ConversionRequest>,
    /// Contains conversion response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub response: Option<ConversionResponse>,
}

/// Part of ConversionReview which is set on input (i.e. generated by apiserver)
#[derive(Serialize, Deserialize)]
pub struct ConversionRequest {
    /// Copied from the corresponding consructing [`ConversionReview`].
    /// This field is not part of the Kubernetes API, it's consumed only by `kube`.
    #[serde(skip)]
    pub types: Option<TypeMeta>,
    /// Random uid uniquely identifying this conversion call
    pub uid: String,
    /// The API group and version the objects should be converted to
    #[serde(rename = "desiredAPIVersion")]
    pub desired_api_version: String,
    /// The list of objects to convert.
    /// May contain one or more objects, in one or more versions.
    // This field used raw Value instead of Object/DynamicObject to simplify
    // further downcasting.
    pub objects: Vec<serde_json::Value>,
}

impl ConversionRequest {
    /// Extracts request from the [`ConversionReview`].
    /// Fails if review has missing request.
    pub fn from_review(review: ConversionReview) -> Result<Self, ConvertConversionReviewError> {
        match review.request {
            Some(mut req) => {
                req.types = Some(review.types);
                Ok(req)
            }
            None => Err(ConvertConversionReviewError),
        }
    }
}

/// Part of ConversionReview which is set on output (i.e. generated by conversion webhook)
#[derive(Serialize, Deserialize)]
pub struct ConversionResponse {
    /// Copied from the corresponding consructing [`ConversionRequest`].
    /// This field is not part of the Kubernetes API, it's consumed only by `kube`.
    #[serde(skip)]
    pub types: Option<TypeMeta>,
    /// Copy of .request.uid
    pub uid: String,
    /// Outcome of the conversion operation.
    /// Success: all objects were successfully converted
    /// Failure: at least one object could not be converted.
    /// It is recommended that conversion fails as rare as possible.
    pub result: Status,
    /// Converted objects in the same order as in the request. Should be empty
    /// if conversion failed.
    #[serde(rename = "convertedObjects")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converted_objects: Option<Vec<serde_json::Value>>,
}

impl ConversionResponse {
    /// Creates successful conversion response.
    /// `converted_objects` must specify objects in the exact same order as on input.
    pub fn success(request: ConversionRequest, converted_objects: Vec<serde_json::Value>) -> Self {
        ConversionResponse {
            types: request.types,
            uid: request.uid,
            result: Status::success(),
            converted_objects: Some(converted_objects),
        }
    }

    /// Creates failed conversion response (discouraged).
    /// `request_uid` must be equal to the `.uid` field in the request.
    /// `message` and `reason` will be returned to the apiserver.
    pub fn error(request: ConversionRequest, message: &str, reason: &str) -> Self {
        ConversionResponse {
            types: request.types,
            uid: request.uid,
            result: Status::failure(message, reason),
            converted_objects: None,
        }
    }

    /// Creates failed conversion response, not matched with any request.
    /// You should only call this function when request couldn't be parsed into [`ConversionRequest`].
    /// Otherwise use `error`.
    pub fn unmatched_error(message: &str, reason: &str) -> Self {
        ConversionResponse {
            types: None,
            uid: String::new(),
            result: Status::failure(message, reason),
            converted_objects: None,
        }
    }

    /// Converts response into a [`ConversionReview`] value, ready to be sent as a response.
    pub fn into_review(mut self) -> ConversionReview {
        ConversionReview {
            types: self.types.take().unwrap_or_else(|| {
                // we don't know which uid, apiVersion and kind to use, let's just use something
                TypeMeta {
                    api_version: META_API_VERSION_V1.to_string(),
                    kind: META_KIND.to_string(),
                }
            }),
            request: None,
            response: Some(self),
        }
    }
}
