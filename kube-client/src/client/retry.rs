//! Retry policy for Kubernetes API requests.
//!
//! This module provides a [`RetryPolicy`] that implements [`tower::retry::Policy`]
//! for retrying failed Kubernetes API requests with exponential backoff.
//!
//! # Example
//!
//! ```no_run
//! use kube::{Client, Config, client::ConfigExt};
//! use kube::client::retry::RetryPolicy;
//! use tower::{ServiceBuilder, BoxError};
//! use tower::retry::RetryLayer;
//! use tower::buffer::BufferLayer;
//! use hyper_util::rt::TokioExecutor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::infer().await?;
//! let https = config.rustls_https_connector()?;
//!
//! let service = ServiceBuilder::new()
//!     .layer(config.base_uri_layer())
//!     .option_layer(config.auth_layer()?)
//!     .layer(BufferLayer::new(1024))
//!     .layer(RetryLayer::new(RetryPolicy::default()))
//!     .map_err(BoxError::from)
//!     .service(hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(https));
//!
//! let client = Client::new(service, config.default_namespace);
//! # Ok(())
//! # }
//! ```

use std::time::Duration;

use http::{Request, Response, StatusCode};
use tower::{
    BoxError,
    retry::{
        Policy,
        backoff::{Backoff, ExponentialBackoff, ExponentialBackoffMaker, MakeBackoff},
    },
    util::rng::HasherRng,
};

use super::Body;

/// Backoff configuration validation error.
pub use tower::retry::backoff::InvalidBackoff;

/// A retry policy for Kubernetes API requests.
///
/// This policy retries requests that fail with:
/// - 429 Too Many Requests
/// - 503 Service Unavailable
/// - 504 Gateway Timeout
///
/// Uses exponential backoff starting from `min_delay` up to `max_delay`,
/// with a configurable maximum number of retries.
#[derive(Clone)]
pub struct RetryPolicy {
    backoff: ExponentialBackoff,
    current_attempt: u32,
    max_retries: u32,
}

impl RetryPolicy {
    /// Create a new retry policy with custom parameters.
    ///
    /// # Arguments
    ///
    /// * `min_delay` - Initial delay between retries
    /// * `max_delay` - Maximum delay between retries (cap for exponential growth)
    /// * `max_retries` - Maximum number of retry attempts
    ///
    /// # Errors
    ///
    /// Returns [`InvalidBackoff`] if the backoff parameters are invalid.
    pub fn new(min_delay: Duration, max_delay: Duration, max_retries: u32) -> Result<Self, InvalidBackoff> {
        let backoff =
            ExponentialBackoffMaker::new(min_delay, max_delay, 2.0, HasherRng::new())?.make_backoff();

        Ok(Self {
            backoff,
            current_attempt: 0,
            max_retries,
        })
    }

    /// Check if the status code is retryable.
    fn is_retryable_status(status: StatusCode) -> bool {
        matches!(
            status,
            StatusCode::TOO_MANY_REQUESTS | StatusCode::SERVICE_UNAVAILABLE | StatusCode::GATEWAY_TIMEOUT
        )
    }
}

impl Default for RetryPolicy {
    /// Create a default retry policy.
    ///
    /// Default parameters:
    /// - `min_delay`: 500ms
    /// - `max_delay`: 5s
    /// - `max_retries`: 3
    fn default() -> Self {
        Self::new(Duration::from_millis(500), Duration::from_secs(5), 3)
            .expect("default RetryPolicy parameters are valid")
    }
}

impl<Res> Policy<Request<Body>, Response<Res>, BoxError> for RetryPolicy {
    type Future = tokio::time::Sleep;

    fn retry(
        &mut self,
        _req: &mut Request<Body>,
        result: &mut Result<Response<Res>, BoxError>,
    ) -> Option<Self::Future> {
        match result {
            Ok(response)
                if Self::is_retryable_status(response.status())
                    && self.current_attempt < self.max_retries =>
            {
                self.current_attempt += 1;
                Some(self.backoff.next_backoff())
            }
            _ => None,
        }
    }

    fn clone_request(&mut self, req: &Request<Body>) -> Option<Request<Body>> {
        // Try to clone the body - only Kind::Once bodies can be cloned
        let body = req.body().try_clone()?;

        let mut builder = Request::builder()
            .method(req.method().clone())
            .uri(req.uri().clone())
            .version(req.version());

        // Copy headers
        if let Some(headers) = builder.headers_mut() {
            headers.extend(req.headers().clone());
        }

        // Copy extensions
        builder.body(body).ok().map(|mut new_req| {
            *new_req.extensions_mut() = req.extensions().clone();
            new_req
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_retries, 3);
    }

    #[test]
    fn test_retryable_status() {
        assert!(RetryPolicy::is_retryable_status(StatusCode::TOO_MANY_REQUESTS));
        assert!(RetryPolicy::is_retryable_status(StatusCode::SERVICE_UNAVAILABLE));
        assert!(RetryPolicy::is_retryable_status(StatusCode::GATEWAY_TIMEOUT));

        assert!(!RetryPolicy::is_retryable_status(StatusCode::OK));
        assert!(!RetryPolicy::is_retryable_status(StatusCode::BAD_REQUEST));
        assert!(!RetryPolicy::is_retryable_status(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(!RetryPolicy::is_retryable_status(StatusCode::NOT_FOUND));
    }

    #[test]
    fn test_invalid_backoff() {
        let result = RetryPolicy::new(Duration::from_secs(10), Duration::from_secs(1), 3);
        assert!(result.is_err());
    }
}
