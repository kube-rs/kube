#![deny(clippy::all)]
#![deny(clippy::pedantic)]
// Makes for confusing SNAFU context selectors
#![allow(clippy::pub_enum_variant_names)]
// Triggered by many derive macros (kube-derive, derivative)
#![allow(clippy::default_trait_access)]

pub mod controller;
mod manager;
pub mod reflector;
pub mod scheduler;
pub mod utils;
pub mod watcher;

pub use controller::{controller, ControllerBuilder};
pub use reflector::reflector;
pub use scheduler::scheduler;
pub use watcher::watcher;
