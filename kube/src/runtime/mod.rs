//! Legacy runtime helpers for keeping track of Kubernetes resources
//!
//! Please see the `kube-runtime` crate for the replacement of these.
mod informer;
mod reflector;

pub use informer::Informer;
pub use reflector::Reflector;
