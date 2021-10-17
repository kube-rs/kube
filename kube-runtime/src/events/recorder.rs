use crate::events::{EventSource, EventType, NewEvent};
use k8s_openapi::{
    api::{core::v1::ObjectReference, events::v1::Event},
    apimachinery::pkg::apis::meta::v1::{MicroTime, ObjectMeta},
    chrono::Utc,
};
use kube::{api::PostParams, Api, Client};

#[derive(Clone)]
/// A publisher abstraction to emit Kubernetes' events.
///
/// All events emitted by an `EventRecorder` are attached to the [`ObjectReference`]
/// specified when building the recorder using [`EventRecorder::new`].
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::{EventSource, EventRecorder, NewEvent, EventType};
/// use k8s_openapi::api::core::v1::ObjectReference;
///
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let k8s_client: kube::Client = todo!();
/// let event_source = EventSource {
///     controller_pod_name: "my-awesome-controller-abcdef".try_into().unwrap(),
///     controller_name: "my-awesome-controller".into(),
/// };
///
/// // You can populate this using `ObjectMeta` and `ApiResource` information
/// // from the object you are working with.
/// let object_reference = ObjectReference {
///     // [...]
///     # ..Default::default()
/// };
///
/// let event_recorder = EventRecorder::new(k8s_client, event_source, object_reference);
/// event_recorder.publish(NewEvent {
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
pub struct EventRecorder {
    event_client: Api<Event>,
    event_source: EventSource,
    object_reference: ObjectReference,
}

impl EventRecorder {
    /// Build a new [`EventRecorder`] instance to emit events attached to the
    /// specified [`ObjectReference`].
    pub fn new(k8s_client: Client, event_source: EventSource, object_reference: ObjectReference) -> Self {
        let event_client = match object_reference.namespace.as_ref() {
            None => Api::all(k8s_client),
            Some(namespace) => Api::namespaced(k8s_client, namespace),
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
    pub async fn publish(&self, new_event: NewEvent) -> Result<(), kube::Error> {
        self.event_client
            .create(&PostParams::default(), &Event {
                action: Some(new_event.action),
                reason: Some(new_event.reason),
                deprecated_count: None,
                deprecated_first_timestamp: None,
                deprecated_last_timestamp: None,
                deprecated_source: None,
                event_time: MicroTime(Utc::now()),
                regarding: Some(self.object_reference.clone()),
                note: new_event.note,
                metadata: ObjectMeta {
                    namespace: Some(self.object_reference.namespace.clone().unwrap()),
                    generate_name: Some(format!("{}-", self.event_source.controller_name)),
                    ..Default::default()
                },
                reporting_controller: Some(self.event_source.controller_name.clone()),
                reporting_instance: Some(self.event_source.controller_pod_name.clone().into()),
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
