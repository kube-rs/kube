use k8s_openapi::api::core::v1::ObjectReference;
use std::{convert::TryFrom, fmt::Formatter};

/// Required information to publish a new event via [`EventRecorder::publish`].
///
/// [`EventRecorder::publish`]: crate::events::EventRecorder::publish
pub struct NewEvent {
    /// The action that was taken (either successfully or unsuccessfully) against
    /// the references object.
    ///
    /// `action` must be machine-readable.
    pub action: EventAction,
    /// The reason explaining why the `action` was taken.
    ///
    /// `reason` must be human-readable.
    pub reason: EventReason,
    /// A optional description of the status of the `action`.
    ///
    /// `note` must be human-readable.
    pub note: Option<EventNote>,
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
    pub secondary_object: Option<ObjectReference>,
}

/// The event severity or type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EventType {
    /// An event took place - nothing to worry about.
    Normal,
    /// Something is not working as expected - it might be worth to have a look.
    Warning,
}

/// Details about the event emitter.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::EventSource;
///
/// let event_source = EventSource {
///     controller_pod: "my-awesome-controller-abcdef".try_into().unwrap(),
///     controller: "my-awesome-controller".into(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventSource {
    /// The name of the controller publishing the event.
    ///
    /// E.g. `my-awesome-controller`.
    ///
    /// # Naming note
    ///
    /// `controller_name` is mapped to `reportingController` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    pub controller: String,
    /// The name of the controller pod publishing the event.
    ///
    /// E.g. `my-awesome-controller-abcdef`.
    ///
    /// The name of the controller pod can be retrieved using Kubernetes' API or
    /// it can injected as an environment variable using
    ///
    /// ```yaml
    /// env:
    ///   - name: CONTROLLER_POD_NAME
    ///     valueFrom:
    ///       fieldRef:
    ///         fieldPath: metadata.name
    /// ```
    ///
    /// in the manifest of your controller.
    ///
    /// # Naming note
    ///
    /// `controller_pod_name` is mapped to `reportingInstance` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    pub controller_pod: ControllerPodName,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The name of the controller pod publishing the event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::ControllerPodName;
///
/// let controller_pod_name: ControllerPodName = "my-awesome-controller-abcdef".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - shorter than 128 characters.
pub struct ControllerPodName(String);

impl TryFrom<&str> for ControllerPodName {
    type Error = ControllerPodNameParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for ControllerPodName {
    type Error = ControllerPodNameParsingError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        let n_chars = v.chars().count();
        if n_chars > 128 {
            Err(ControllerPodNameParsingError {
                controller_pod_name: v,
            })
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for ControllerPodName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for ControllerPodName {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ControllerPodNameParsingError {
    controller_pod_name: String,
}

impl std::fmt::Display for ControllerPodNameParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "The controller pod name must be shorter than 128 characters.")
    }
}

impl std::error::Error for ControllerPodNameParsingError {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The reason for an action that led to a published event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::EventReason;
///
/// let reason: EventReason = "Scheduling".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - shorter than 128 characters.
pub struct EventReason(String);

impl TryFrom<&str> for EventReason {
    type Error = EventReasonParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for EventReason {
    type Error = EventReasonParsingError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        let n_chars = v.chars().count();
        if n_chars > 128 {
            Err(EventReasonParsingError { reason: v })
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for EventReason {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for EventReason {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventReasonParsingError {
    reason: String,
}

impl std::fmt::Display for EventReasonParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "The reason for an event must be shorter than 128 characters.")
    }
}

impl std::error::Error for EventReasonParsingError {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The action taken by the controller that led to a published event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::EventAction;
///
/// let reason: EventAction = "Pulling".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - shorter than 128 characters.
pub struct EventAction(String);

impl TryFrom<&str> for EventAction {
    type Error = EventActionParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for EventAction {
    type Error = EventActionParsingError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        let n_chars = v.chars().count();
        if n_chars > 128 {
            Err(EventActionParsingError { action: v })
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for EventAction {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for EventAction {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventActionParsingError {
    action: String,
}

impl std::fmt::Display for EventActionParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "The action for an event must be shorter than 128 characters.")
    }
}

impl std::error::Error for EventActionParsingError {}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
/// The human-readable message attached to a published event.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::EventNote;
///
/// let note: EventNote = "Pulling `nginx` Docker image from DockerHub.".try_into().unwrap();
/// ```
///
/// It must be:
///
/// - smaller than 1 kilobyte.
pub struct EventNote(String);

impl TryFrom<&str> for EventNote {
    type Error = EventNoteParsingError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::try_from(value.to_string())
    }
}

impl TryFrom<String> for EventNote {
    type Error = EventNoteParsingError;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        // Limit imposed by Kubernetes' API
        if v.len() > 1024 {
            Err(Self::Error { note: v })
        } else {
            Ok(Self(v))
        }
    }
}

impl AsRef<str> for EventNote {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Into<String> for EventNote {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct EventNoteParsingError {
    note: String,
}

impl std::fmt::Display for EventNoteParsingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "The note for an event must be smaller than 1 kilobyte.")
    }
}

impl std::error::Error for EventNoteParsingError {}
