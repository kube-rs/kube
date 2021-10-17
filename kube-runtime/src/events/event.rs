use crate::events::EventType;
use k8s_openapi::api::core::v1::ObjectReference;

/// Required information to publish a new event via [`EventRecorder::publish`].
///
/// [`EventRecorder::publish`]: crate::events::EventRecorder::publish
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
    /// Some events are emitted for actions that affect multiple objects.
    /// `secondary_object` can be populated to capture this detail.
    ///
    /// For example: the event concerns a `Deployment` and it
    /// affects the current `ReplicaSet` underneath it.
    /// You would therefore populate `secondary_object` using the object
    /// reference of the `ReplicaSet`.
    ///
    /// Set `secondary_object` to `None`, instead, if the event
    /// affects only the object whose reference you passed
    /// to [`EventRecorder::new`].
    ///
    /// # Naming note
    ///
    /// `secondary_object` is mapped to `related` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    ///
    /// [`EventRecorder::new`]: crate::events::EventRecorder::new
    pub secondary_object: Option<ObjectReference>
}
