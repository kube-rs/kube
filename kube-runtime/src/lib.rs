//! Common components for building Kubernetes operators
//!
//! This crate contains the core building blocks to allow users to build
//! controllers/operators/watchers that need to synchronize/reconcile kubernetes
//! state.
//!
//! Newcomers are recommended to start with the [`Controller`] builder, which gives an
//! opinionated starting point that should be appropriate for simple operators, but all
//! components are designed to be usable á la carte if your operator doesn't quite fit that mold.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// Triggered by many derive macros (kube-derive, derivative)
#![allow(clippy::default_trait_access)]
#![allow(clippy::type_repetition_in_bounds)]
// Triggered by Tokio macros
#![allow(clippy::semicolon_if_nothing_returned)]

pub mod controller;
k8s_openapi::k8s_if_ge_1_19! {
    pub mod events;
}
pub mod finalizer;
pub mod reflector;
pub mod scheduler;
pub mod utils;
pub mod wait;
pub mod watcher;

pub use controller::{applier, Controller};
pub use finalizer::finalizer;
pub use reflector::reflector;
pub use scheduler::scheduler;
pub use utils::WatchStreamExt;
pub use watcher::watcher;
