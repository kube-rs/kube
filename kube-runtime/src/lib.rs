//! Crate with kubernetes runtime components
//!
//! This crate contains the core building blocks to allow users to build
//! controllers/operators/watchers that need to synchronize/reconcile kubernetes
//! state.
//!
//! Newcomers should generally get started with the `Controller` builder, which manages
//! all state internals for you.

#![deny(unsafe_code)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// Makes for confusing SNAFU context selectors
#![allow(clippy::pub_enum_variant_names)]
// Triggered by many derive macros (kube-derive, derivative)
#![allow(clippy::default_trait_access)]

pub mod controller;
pub mod reflector;
pub mod scheduler;
pub mod utils;
pub mod watcher;

pub use controller::{applier, Controller};
pub use reflector::{reflector, Store};
pub use scheduler::scheduler;
pub use watcher::watcher;
