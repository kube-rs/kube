//! Contains types useful for implementing custom resource conversion webhooks.

pub use self::types::{
    ConversionRequest, ConversionResponse, ConversionReview, ConvertConversionReviewError,
};

/// Defines low-level typings.
mod types;
