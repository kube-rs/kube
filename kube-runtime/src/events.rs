//! Publishes events for objects
use k8s_openapi::{
    api::{core::v1::ObjectReference, events::v1::Event},
    apimachinery::pkg::apis::meta::v1::{MicroTime, ObjectMeta},
    chrono::Utc,
};
use kube_client::{
    api::{Api, PostParams},
    Client,
};

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
/// use kube_runtime::events::EventSource;
///
/// let event_source = EventSource {
///     controller_pod: "my-awesome-controller-abcdef".into(),
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
    /// it can be injected as an environment variable using
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
    /// `controller_pod` is mapped to `reportingInstance` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    pub controller_pod: String,
}


/// A publisher abstraction to emit Kubernetes' events.
///
/// All events emitted by an `EventRecorder` are attached to the [`ObjectReference`]
/// specified when building the recorder using [`EventRecorder::new`].
///
/// ```rust
/// use kube::runtime::events::{EventSource, EventRecorder, NewEvent, EventType};
/// use k8s_openapi::api::core::v1::ObjectReference;
///
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let client: kube::Client = todo!();
/// let source = EventSource {
///     controller_pod: "my-awesome-controller-abcdef".into(),
///     controller: "my-awesome-controller".into(),
/// };
///
/// // You can populate this using `ObjectMeta` and `ApiResource` information
/// // from the object you are working with.
/// let object_reference = ObjectReference {
///     // [...]
///     # ..Default::default()
/// };
///
/// let recorder = EventRecorder::new(client, source, object_reference);
/// recorder.publish(NewEvent {
///     action: "Scheduling".into(),
///     reason: "Pulling".into(),
///     note: Some("Pulling image `nginx`".into()),
///     event_type: EventType::Normal,
///     secondary_object: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
///
/// Events attached to an object will be shown in the `Events` section of the output of
/// of `kubectl describe` for that object.
#[derive(Clone)]
pub struct EventRecorder {
    events: Api<Event>,
    source: EventSource,
    reference: ObjectReference,
}

impl EventRecorder {
    /// Build a new [`EventRecorder`] instance to emit events attached to the
    /// specified [`ObjectReference`].
    #[must_use]
    pub fn new(client: Client, source: EventSource, reference: ObjectReference) -> Self {
        let events = match reference.namespace.as_ref() {
            None => Api::all(client),
            Some(namespace) => Api::namespaced(client, namespace),
        };
        Self {
            events,
            source,
            reference,
        }
    }

    /// Publish a new Kubernetes' event.
    ///
    /// # Access control
    ///
    /// The event object is created in the same namespace of the [`ObjectReference`]
    /// you specified in [`EventRecorder::new`].
    /// Make sure that your controller has `create` permissions in the required namespaces
    /// for the `event` resource in the API group `events.k8s.io`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](`kube_client::Error`) if the event is rejected by Kubernetes.
    pub async fn publish(&self, ev: NewEvent) -> Result<(), kube_client::Error> {
        self.events
            .create(&PostParams::default(), &Event {
                action: Some(ev.action),
                reason: Some(ev.reason),
                deprecated_count: None,
                deprecated_first_timestamp: None,
                deprecated_last_timestamp: None,
                deprecated_source: None,
                event_time: MicroTime(Utc::now()),
                regarding: Some(self.reference.clone()),
                note: ev.note.map(Into::into),
                metadata: ObjectMeta {
                    namespace: self.reference.namespace.clone(),
                    generate_name: Some(format!("{}-", self.source.controller)),
                    ..Default::default()
                },
                reporting_controller: Some(self.source.controller.clone()),
                reporting_instance: Some(self.source.controller_pod.clone()),
                series: None,
                type_: match ev.event_type {
                    EventType::Normal => Some("Normal".into()),
                    EventType::Warning => Some("Warning".into()),
                },
                related: ev.secondary_object,
            })
            .await?;
        Ok(())
    }
}
