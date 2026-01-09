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

use std::{fmt, time::Duration};

use http::{Request, Response, StatusCode};
use tower::{BoxError, retry::Policy};

use super::Body;

/// Backoff configuration validation error.
#[derive(Debug)]
pub struct InvalidBackoff(&'static str);

impl fmt::Display for InvalidBackoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid backoff: {}", self.0)
    }
}

impl std::error::Error for InvalidBackoff {}

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
    min_delay: Duration,
    max_delay: Duration,
    max_retries: u32,
    current_attempt: u32,
    current_delay: Duration,
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
    /// Returns [`InvalidBackoff`] if:
    /// - `min_delay` > `max_delay`
    /// - `max_delay` is zero
    pub fn new(min_delay: Duration, max_delay: Duration, max_retries: u32) -> Result<Self, InvalidBackoff> {
        if min_delay > max_delay {
            return Err(InvalidBackoff("min_delay must not exceed max_delay"));
        }
        if max_delay.is_zero() {
            return Err(InvalidBackoff("max_delay must be non-zero"));
        }

        Ok(Self {
            min_delay,
            max_delay,
            max_retries,
            current_attempt: 0,
            current_delay: min_delay,
        })
    }

    /// Reset the policy state for a new request sequence.
    fn reset(&mut self) {
        self.current_attempt = 0;
        self.current_delay = self.min_delay;
    }

    /// Advance to the next retry attempt, returning the delay to wait.
    fn next_backoff(&mut self) -> Duration {
        let delay = self.current_delay;
        // Exponential backoff: double the delay, capped at max_delay
        self.current_delay = std::cmp::min(self.current_delay * 2, self.max_delay);
        self.current_attempt += 1;
        delay
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
            Ok(response) if Self::is_retryable_status(response.status()) => {
                if self.current_attempt < self.max_retries {
                    let delay = self.next_backoff();
                    tracing::debug!(
                        status = %response.status(),
                        attempt = self.current_attempt,
                        delay_ms = delay.as_millis(),
                        "Retrying request"
                    );
                    Some(tokio::time::sleep(delay))
                } else {
                    tracing::debug!(
                        status = %response.status(),
                        attempts = self.current_attempt,
                        "Max retries exceeded"
                    );
                    self.reset();
                    None
                }
            }
            Ok(_) => {
                // Successful response, reset for next request
                self.reset();
                None
            }
            Err(err) => {
                // Don't retry on errors - they might not be idempotent
                tracing::debug!(error = %err, "Request failed with error, not retrying");
                self.reset();
                None
            }
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
        assert_eq!(policy.min_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(5));
        assert_eq!(policy.max_retries, 3);
    }

    #[test]
    fn test_exponential_backoff() {
        let mut policy = RetryPolicy::new(Duration::from_millis(100), Duration::from_secs(1), 5).unwrap();

        assert_eq!(policy.next_backoff(), Duration::from_millis(100));
        assert_eq!(policy.next_backoff(), Duration::from_millis(200));
        assert_eq!(policy.next_backoff(), Duration::from_millis(400));
        assert_eq!(policy.next_backoff(), Duration::from_millis(800));
        // Capped at max_delay
        assert_eq!(policy.next_backoff(), Duration::from_secs(1));
        assert_eq!(policy.next_backoff(), Duration::from_secs(1));
    }

    #[test]
    fn test_reset() {
        let mut policy = RetryPolicy::default();
        policy.next_backoff();
        policy.next_backoff();
        assert_eq!(policy.current_attempt, 2);

        policy.reset();
        assert_eq!(policy.current_attempt, 0);
        assert_eq!(policy.current_delay, policy.min_delay);
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
    fn test_invalid_min_exceeds_max() {
        let result = RetryPolicy::new(Duration::from_secs(10), Duration::from_secs(1), 3);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_zero_max_delay() {
        let result = RetryPolicy::new(Duration::ZERO, Duration::ZERO, 3);
        assert!(result.is_err());
    }
}
