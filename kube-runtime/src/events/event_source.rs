use crate::events::ControllerPodName;

/// Details about the event emitter.
///
/// ```rust
/// use std::convert::TryInto;
/// use kube_runtime::events::EventSource;
///
/// let event_source = EventSource {
///     controller_pod_name: "my-awesome-controller-abcdef".try_into().unwrap(),
///     controller_name: "my-awesome-controller".into(),
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
    pub controller_name: String,
    /// The name of the controller pod publishing the event.
    ///
    /// E.g. `my-awesome-controller-abcdef`.
    ///
    /// # Naming note
    ///
    /// `controller_pod_name` is mapped to `reportingInstance` in
    /// [`Events API`](https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.22/#event-v1-events-k8s-io).
    pub controller_pod_name: ControllerPodName,
}
