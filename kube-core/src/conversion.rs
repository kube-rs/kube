//! Contains utilities for implements custom resource conversion webhooks.
//! Abstractions in this module and its submodules may be ordered:
//! - `low_level` submodule contains raw typings, allowing for full control (probably you don't need these unless you are implementing your own apiserver).
//! - `Conversion` together with `ConversionHandler` are middle ground: they abstract you from low-level details but not require particular conversion strategy.
//! - `StarConversion` is opinionated strategy, where all conversions are done using intermediate unversioned representation as a middle point.
//! Third option is preferred - this is how Kubernetes implements versioning and convertion for builtin APIs.

use std::sync::Arc;

use self::low_level::{ConversionRequest, ConversionResponse, ConversionReview};

/// Defines low-level typings.
pub mod low_level;
/// High-level easy-to-use conversion.
mod star;

pub use star::{StarConversion, StarConversionBuilder, StarRay};

/// Helper which implements low-level API based on the provided converter.
#[derive(Clone)]
pub struct ConversionHandler {
    conversion: Arc<dyn Conversion + Send + Sync>,
}

impl ConversionHandler {
    /// Creates new `ConversionHandler` which will use given `conversion` for operation.
    pub fn new<C: Conversion + Send + Sync + 'static>(conversion: C) -> Self {
        ConversionHandler {
            conversion: Arc::new(conversion),
        }
    }

    /// Processes `request` using stored converter so that you don't need
    /// to think about copying `.uid` and other boring things.
    /// Returned value is ready to be serialized and returned.
    pub fn handle(&self, review: ConversionReview) -> ConversionReview {
        let mut req = match ConversionRequest::from_review(review) {
            Ok(r) => r,
            Err(_) => {
                return ConversionResponse::unmatched_error(".request is unset in input", "InvalidRequest")
                    .into_review()
            }
        };

        let mut converted_objects = Vec::new();
        let input_objects = std::mem::take(&mut req.objects);
        for (idx, object) in input_objects.into_iter().enumerate() {
            match self.conversion.convert(object, &req.desired_api_version) {
                Ok(c) => converted_objects.push(c),
                Err(error) => {
                    let msg = format!("Conversion of object {} failed: {}", idx, error);
                    return ConversionResponse::error(req, &msg, "ConversionFailed").into_review();
                }
            }
        }
        ConversionResponse::success(req, converted_objects).into_review()
    }
}

/// Conversion is entity which supports all `N*(N-1)` possible conversion directions.
/// This trait does not specify strategy used. You may implement this trait yourself
/// or use [`StarConversion`](StarConversion).
pub trait Conversion {
    /// Actually performs conversion.
    /// # Requirements
    /// All metadata fields except labels and annotations must not be mutated.
    /// # Errors
    /// While the signature allows returning errors, it is discouraged.
    /// Ideally, conversion should always succeed.
    fn convert(
        &self,
        object: serde_json::Value,
        desired_api_version: &str,
    ) -> Result<serde_json::Value, String>;
}

#[cfg(test)]
mod tests {
    use crate::response::StatusSummary;

    use super::{
        low_level::{ConversionRequest, ConversionReview, META_API_VERSION_V1, META_KIND},
        Conversion, ConversionHandler,
    };

    struct NoopConversion;
    impl Conversion for NoopConversion {
        fn convert(
            &self,
            object: serde_json::Value,
            _desired_api_version: &str,
        ) -> Result<serde_json::Value, String> {
            Ok(object)
        }
    }

    #[test]
    fn test_conversion_handler_upholds_contract() {
        let obj1 = serde_json::json!({
            "foo": true
        });
        let obj2 = serde_json::json!({
            "bar": 6
        });
        let handler = ConversionHandler::new(NoopConversion);

        let input = ConversionReview {
            types: crate::TypeMeta {
                api_version: META_API_VERSION_V1.to_string(),
                kind: META_KIND.to_string(),
            },
            request: Some(ConversionRequest {
                types: None,
                uid: "some-uid".to_string(),
                desired_api_version: "doesnotmatter".to_string(),
                objects: vec![obj1.clone(), obj2.clone()],
            }),
            response: None,
        };

        let output = handler.handle(input);
        assert_eq!(output.types.api_version, META_API_VERSION_V1);
        assert_eq!(output.types.kind, META_KIND);
        let resp = output.response.unwrap();
        assert!(matches!(resp.result.status, Some(StatusSummary::Success)));
        assert!(resp.result.message.is_empty());
        assert_eq!(resp.converted_objects, Some(vec![obj1, obj2]));
    }
}
