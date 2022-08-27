//! Contains utilities for implements custom resource conversion webhooks.
//! Abstractions in this module and its submodules may be ordered:
//! - `low_level` submodule contains raw typings, allowing for full control (probably you don't need these unless you are implementing your own apiserver).
//! - `Converter` together with `ConversionHandler` are middle ground: they abstract you from low-level details but not require particular conversion strategy.
//! - `StarConverter` is opinionated strategy, where all conversions are done using intermediate unversioned representation as a middle point.
//! Third option is preferred - this is how Kubernetes implements versioning and convertion for builtin APIs.

use std::sync::Arc;

use self::low_level::{ConversionResponse, ConversionReview};

/// Defines low-level typings.
pub mod low_level;
/// High-level easy-to-use converter.
mod star;

pub use star::{StarConverter, StarConverterBuilder, StarRay};

/// Helper which implements low-level API based on the provided converter.
#[derive(Clone)]
pub struct ConversionHandler {
    converter: Arc<dyn Converter + Send + Sync>,
}

impl ConversionHandler {
    /// Creates new `ConversionHandler` which will use given `converter` for operation.
    pub fn new<C: Converter + Send + Sync + 'static>(converter: C) -> Self {
        ConversionHandler {
            converter: Arc::new(converter),
        }
    }

    /// Processes `request` using stored converter so that you don't need
    /// to think about copying `.uid` and other boring things.
    /// Returned value is ready to be serialized and returned.
    pub fn handle(&self, review: ConversionReview) -> ConversionReview {
        let req = match review.request {
            Some(r) => r,
            None => {
                return ConversionReview {
                    types: review.types.clone(),
                    request: None,
                    response: Some(ConversionResponse::error(
                        String::new(),
                        ".request is unset in input".to_string(),
                    )),
                }
            }
        };

        let mut converted_objects = Vec::new();
        for (idx, object) in req.objects.into_iter().enumerate() {
            match self.converter.convert(object, &req.desired_api_version) {
                Ok(c) => converted_objects.push(c),
                Err(error) => {
                    let msg = format!("Conversion of object {} failed: {}", idx, error);
                    return ConversionReview {
                        types: review.types.clone(),
                        request: None,
                        response: Some(ConversionResponse::error(req.uid, msg)),
                    };
                }
            }
        }
        ConversionReview {
            types: review.types.clone(),
            request: None,
            response: Some(ConversionResponse::success(req.uid, converted_objects)),
        }
    }
}

/// Converter is entity which supports all `N*(N-1)` possible conversion directions.
/// This trait does not specify strategy used. You may implement this trait yourself
/// or use [`StarConverter`](StarConverter).
pub trait Converter {
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
    use crate::conversion::low_level::ConversionStatus;

    use super::{
        low_level::{ConversionRequest, ConversionReview, META_API_VERSION_V1, META_KIND},
        ConversionHandler, Converter,
    };

    struct NoopConverter;
    impl Converter for NoopConverter {
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
        let handler = ConversionHandler::new(NoopConverter);

        let input = ConversionReview {
            types: crate::TypeMeta {
                api_version: META_API_VERSION_V1.to_string(),
                kind: META_KIND.to_string(),
            },
            request: Some(ConversionRequest {
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
        assert!(matches!(resp.result.status, Some(ConversionStatus::Success)));
        assert!(resp.result.message.is_none());
        assert_eq!(resp.converted_objects, Some(vec![obj1, obj2]));
    }
}
