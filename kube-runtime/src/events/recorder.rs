use crate::events::{EventSource, EventType, NewEvent};
use k8s_openapi::{
    api::{core::v1::ObjectReference, events::v1::Event},
    apimachinery::pkg::apis::meta::v1::{MicroTime, ObjectMeta},
    chrono::Utc,
};
use kube_client::{api::{PostParams, Api}, Client};

#[derive(Clone)]
/// A publisher abstraction to emit Kubernetes' events.
///
/// All events emitted by an `EventRecorder` are attached to the [`ObjectReference`]
/// specified when building the recorder using [`EventRecorder::new`].
///
/// ```rust
/// use std::convert::TryInto;
/// use kube::runtime::events::{EventSource, EventRecorder, NewEvent, EventType};
/// use k8s_openapi::api::core::v1::ObjectReference;
///
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let client: kube::Client = todo!();
/// let event_source = EventSource {
///     controller_pod: "my-awesome-controller-abcdef".try_into().unwrap(),
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
/// let recorder = EventRecorder::new(client, event_source, object_reference);
/// recorder.publish(NewEvent {
///     action: "Scheduling".try_into()?,
///     reason: "Pulling".try_into()?,
///     note: Some("Pulling image `nginx`".try_into()?),
///     event_type: EventType::Normal,
///     secondary_object: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
///
/// Events attached to an object will be shown in the `Events` section of the output of
/// of `kubectl describe` for that object.
pub struct EventRecorder {
    event_client: Api<Event>,
    event_source: EventSource,
    object_reference: ObjectReference,
}

impl EventRecorder {
    /// Build a new [`EventRecorder`] instance to emit events attached to the
    /// specified [`ObjectReference`].
    #[must_use]
    pub fn new(client: Client, event_source: EventSource, object_reference: ObjectReference) -> Self {
        let event_client = match object_reference.namespace.as_ref() {
            None => Api::all(client),
            Some(namespace) => Api::namespaced(client, namespace),
        };
        Self {
            event_client,
            event_source,
            object_reference,
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
    /// Returns an [`Error`](`kube::Error`) if the event is rejected by Kubernetes.
    pub async fn publish(&self, new_event: NewEvent) -> Result<(), kube_client::Error> {
        self.event_client
            .create(&PostParams::default(), &Event {
                action: Some(new_event.action.into()),
                reason: Some(new_event.reason.into()),
                deprecated_count: None,
                deprecated_first_timestamp: None,
                deprecated_last_timestamp: None,
                deprecated_source: None,
                event_time: MicroTime(Utc::now()),
                regarding: Some(self.object_reference.clone()),
                note: new_event.note.map(Into::into),
                metadata: ObjectMeta {
                    namespace: self.object_reference.namespace.clone(),
                    generate_name: Some(format!("{}-", self.event_source.controller)),
                    ..Default::default()
                },
                reporting_controller: Some(self.event_source.controller.clone()),
                reporting_instance: Some(self.event_source.controller_pod.clone().into()),
                series: None,
                type_: match new_event.event_type {
                    EventType::Normal => Some("Normal".into()),
                    EventType::Warning => Some("Warning".into()),
                },
                related: new_event.secondary_object,
            })
            .await?;
        Ok(())
    }
}
