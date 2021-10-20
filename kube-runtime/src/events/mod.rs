//! Publishes events for objects
#![allow(clippy::module_name_repetitions)]

pub use event::{ControllerPodName, EventAction, EventNote, EventReason, EventSource, EventType, NewEvent};
pub use recorder::EventRecorder;

mod event;
mod recorder;
