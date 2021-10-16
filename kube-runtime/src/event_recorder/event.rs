use crate::event_recorder::EventType;

/// Required information to publish a new event via [`EventRecorder::publish`].
///
/// [`EventRecorder::publish`]: crate::event_recorder::EventRecorder::publish
pub struct NewEvent {
    /// The action that was taken (either successfully or unsuccessfully) against
    /// the references object.
    ///
    /// `action` must be machine-readable.
    pub action: String,
    /// The reason explaining why the `action` was taken.
    ///
    /// `reason` must be human-readable.
    pub reason: String,
    /// A optional description of the status of the `action`.
    ///
    /// `note` must be human-readable.
    pub note: Option<String>,
    /// The event severity.
    pub event_type: EventType,
}
