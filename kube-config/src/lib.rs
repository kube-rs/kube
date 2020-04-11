//! Crate for interacting with various Kubernetes config formats
//!
//! This crate contains ways to parse the ~/.kube/config, as well as the incluster
//! service account tokens injected as environment variables.
//!
//! Optionally, oauth support is available
//!
//! # Example
//!
//! The following example will infer what environment you are in, and construct
//! a `Config` based on this. A `Config` can be plugged into the `kube` crate
//! to construct a `kube::Client`.
//!
//! ```rust,no_run
//! use kube_client::Config;
//! use kube::Client;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), kube::Error> {
//!     let config = Config::infer()?
//!     let client = Client::from(config).await?
//!     Ok(())
//! }
//! ```
//!
//! While it's recommended to use `Config::infer()`, it is possible to construct
//! one yourself.

#![deny(missing_docs)]

#[macro_use] extern crate log;

pub mod config;

pub mod error;
mod oauth2;

#[doc(inline)] pub use config::Config;
#[doc(inline)] pub use error::Error;

/// Convient alias for `Result<T, Error>`
pub type Result<T> = std::result::Result<T, Error>;
