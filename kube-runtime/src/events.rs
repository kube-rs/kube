//! Publishes events for objects for kubernetes >= 1.19
use k8s_openapi::{
    api::{core::v1::ObjectReference, events::v1::Event as CoreEvent},
    apimachinery::pkg::apis::meta::v1::{MicroTime, ObjectMeta},
    chrono::Utc,
};
use kube_client::{
    api::{Api, PostParams},
    Client,
};

/// Minimal event type for publishing through [`Recorder::publish`].
///
/// All string fields must be human readable.
pub struct Event {
    /// The event severity.
    ///
    /// Shows up in `kubectl describe` as `Type`.
    pub type_: EventType,

    /// The short reason explaining why the `action` was taken.
    ///
    /// This must be at most 128 characters, and is often PascalCased. Shows up in `kubectl describe` as `Reason`.
    pub reason: String,

    /// A optional description of the status of the `action`.
    ///
    /// This must be at most 1kB in size. Shows up in `kubectl describe` as `Message`.
    pub note: Option<String>,

    /// The action that was taken (either successfully or unsuccessfully) against main object
    ///
    /// This must be at most 128 characters. It does not currently show up in `kubectl describe`.
    pub action: String,

    /// Optional secondary object related to the main object
    ///
    /// Some events are emitted for actions that affect multiple objects.
    /// `secondary` can be populated to capture this detail.
    ///
    /// For example: the event concerns a `Deployment` and it affects the current `ReplicaSet` underneath it.
    /// You would therefore populate `events` using the object reference of the `ReplicaSet`.
    ///
    /// Set `secondary` to `None`, instead, if the event affects only the object whose reference
    /// you passed to [`Recorder::new`].
    ///
    /// # Naming note
    ///
    /// `secondary` is mapped to `related` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    ///
    /// [`Recorder::new`]: crate::events::Recorder::new
    pub secondary: Option<ObjectReference>,
}

/// The event severity or type.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum EventType {
    /// An event took place - nothing to worry about.
    Normal,
    /// Something is not working as expected - it might be worth to have a look.
    Warning,
}

/// Information about the reporting controller.
///
/// ```
/// use kube::runtime::events::Reporter;
///
/// let reporter = Reporter {
///     controller: "my-awesome-controller".into(),
///     instance: std::env::var("CONTROLLER_POD_NAME").ok(),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Reporter {
    /// The name of the reporting controller that is publishing the event.
    ///
    /// This is likely your deployment.metadata.name.
    pub controller: String,

    /// The id of the controller publishing the event. Likely your pod name.
    ///
    /// Useful when running more than one replica on your controller and you need to disambiguate
    /// where events came from.
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
    /// NB: If no `instance` is provided, then `reporting_instance == reporting_controller` in the `Event`.
    pub instance: Option<String>,
}

// simple conversions for when instance == controller
impl From<String> for Reporter {
    fn from(es: String) -> Self {
        Self {
            controller: es,
            instance: None,
        }
    }
}

impl From<&str> for Reporter {
    fn from(es: &str) -> Self {
        Self {
            controller: es.into(),
            instance: None,
        }
    }
}

/// A publisher abstraction to emit Kubernetes' events.
///
/// All events emitted by an `Recorder` are attached to the [`ObjectReference`]
/// specified when building the recorder using [`Recorder::new`].
///
/// ```
/// use kube::{
///   core::Resource,
///   runtime::events::{Reporter, Recorder, Event, EventType}
/// };
/// use k8s_openapi::api::core::v1::ObjectReference;
///
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let client: kube::Client = todo!();
/// let reporter = Reporter {
///     controller: "my-awesome-controller".into(),
///     instance: std::env::var("CONTROLLER_POD_NAME").ok(),
/// };
///
/// // references can be made manually using `ObjectMeta` and `ApiResource`/`Resource` info
/// let reference = ObjectReference {
///     // [...]
///     ..Default::default()
/// };
/// // or for k8s-openapi / kube-derive types, use Resource::object_ref:
/// // let reference = myobject.object_ref();
///
/// let recorder = Recorder::new(client, reporter, reference);
/// recorder.publish(Event {
///     action: "Scheduling".into(),
///     reason: "Pulling".into(),
///     note: Some("Pulling image `nginx`".into()),
///     type_: EventType::Normal,
///     secondary: None,
/// }).await?;
/// # Ok(())
/// # }
/// ```
///
/// Events attached to an object will be shown in the `Events` section of the output of
/// of `kubectl describe` for that object.
#[derive(Clone)]
pub struct Recorder {
    events: Api<CoreEvent>,
    reporter: Reporter,
    reference: ObjectReference,
}

impl Recorder {
    /// Create a new recorder that can publish events for one specific object
    ///
    /// This is intended to be created at the start of your controller's reconcile fn.
    ///
    /// Cluster scoped objects will publish events in the "default" namespace.
    #[must_use]
    pub fn new(client: Client, reporter: Reporter, reference: ObjectReference) -> Self {
        let default_namespace = "kube-system".to_owned(); // default does not work on k8s < 1.22
        let events = Api::namespaced(client, reference.namespace.as_ref().unwrap_or(&default_namespace));
        Self {
            events,
            reporter,
            reference,
        }
    }

    /// Publish a new Kubernetes' event.
    ///
    /// # Access control
    ///
    /// The event object is created in the same namespace of the [`ObjectReference`]
    /// you specified in [`Recorder::new`].
    /// Make sure that your controller has `create` permissions in the required namespaces
    /// for the `event` resource in the API group `events.k8s.io`.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](`kube_client::Error`) if the event is rejected by Kubernetes.
    pub async fn publish(&self, ev: Event) -> Result<(), kube_client::Error> {
        // See https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io
        // for more detail on the fields
        // and what's expected: https://kubernetes.io/docs/reference/using-api/deprecation-guide/#event-v125
        self.events
            .create(&PostParams::default(), &CoreEvent {
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
                    generate_name: Some(format!("{}-", self.reporter.controller)),
                    ..Default::default()
                },
                reporting_controller: Some(self.reporter.controller.clone()),
                reporting_instance: Some(
                    self.reporter
                        .instance
                        .clone()
                        .unwrap_or_else(|| self.reporter.controller.clone()),
                ),
                series: None,
                type_: match ev.type_ {
                    EventType::Normal => Some("Normal".into()),
                    EventType::Warning => Some("Warning".into()),
                },
                related: ev.secondary,
            })
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    #![allow(unused_imports)]

    use k8s_openapi::api::{
        core::v1::{Event as CoreEvent, Service},
        rbac::v1::ClusterRole,
    };
    use kube_client::{Api, Client, Resource};

    use super::{Event, EventType, Recorder};

    #[tokio::test]
    #[ignore] // needs cluster (creates a pointless event on the kubernetes main service)
    async fn event_recorder_attaches_events() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;

        let svcs: Api<Service> = Api::namespaced(client.clone(), "default");
        let s = svcs.get("kubernetes").await?; // always a kubernetes service in default
        let recorder = Recorder::new(client.clone(), "kube".into(), s.object_ref(&()));
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "VeryCoolService".into(),
                note: Some("Sending kubernetes to detention".into()),
                action: "Test event - plz ignore".into(),
                secondary: None,
            })
            .await?;
        let events: Api<CoreEvent> = Api::namespaced(client, "default");

        let event_list = events.list(&Default::default()).await?;
        let found_event = event_list
            .into_iter()
            .find(|e| std::matches!(e.reason.as_deref(), Some("VeryCoolService")))
            .unwrap();
        assert_eq!(found_event.message.unwrap(), "Sending kubernetes to detention");

        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs cluster (creates a pointless event on the kubernetes main service)
    async fn event_recorder_attaches_events_without_namespace() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;

        let svcs: Api<ClusterRole> = Api::all(client.clone());
        let s = svcs.get("system:basic-user").await?; // always get this default ClusterRole
        let recorder = Recorder::new(client.clone(), "kube".into(), s.object_ref(&()));
        recorder
            .publish(Event {
                type_: EventType::Normal,
                reason: "VeryCoolServiceNoNamespace".into(),
                note: Some("Sending kubernetes to detention without namespace".into()),
                action: "Test event - plz ignore".into(),
                secondary: None,
            })
            .await?;
        let events: Api<CoreEvent> = Api::namespaced(client, "kube-system");

        let event_list = events.list(&Default::default()).await?;
        let found_event = event_list
            .into_iter()
            .find(|e| std::matches!(e.reason.as_deref(), Some("VeryCoolServiceNoNamespace")))
            .unwrap();
        assert_eq!(
            found_event.message.unwrap(),
            "Sending kubernetes to detention without namespace"
        );

        Ok(())
    }
}
