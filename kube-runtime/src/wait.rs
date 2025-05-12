//! Waits for objects to reach desired states
use std::{future, pin::pin};

use futures::TryStreamExt;
use kube_client::{Api, Resource};
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use thiserror::Error;

use crate::watcher::{self, watch_object};

#[derive(Debug, Error)]
pub enum Error {
    #[error("failed to probe for whether the condition is fulfilled yet: {0}")]
    ProbeFailed(#[source] watcher::Error),
}

/// Watch an object, and wait for some condition `cond` to return `true`.
///
/// `cond` is passed `Some` if the object is found, otherwise `None`.
///
/// The object is returned when the condition is fulfilled.
///
/// # Caveats
///
/// Keep in mind that the condition is typically fulfilled by an external service, which might not even be available. `await_condition`
/// does *not* automatically add a timeout. If this is desired, wrap it in [`tokio::time::timeout`].
///
/// # Errors
///
/// Fails if the type is not known to the Kubernetes API, or if the [`Api`] does not have
/// permission to `watch` and `list` it.
///
/// Does *not* fail if the object is not found.
///
/// # Usage
///
/// ```
/// use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
/// use kube::{Api, runtime::wait::{await_condition, conditions}};
/// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
/// # let client: kube::Client = todo!();
///
/// let crds: Api<CustomResourceDefinition> = Api::all(client);
/// // .. create or apply a crd here ..
/// let establish = await_condition(crds, "foos.clux.dev", conditions::is_crd_established());
/// let _ = tokio::time::timeout(std::time::Duration::from_secs(10), establish).await?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::missing_panics_doc)] // watch never actually terminates, expect cannot fail
pub async fn await_condition<K>(api: Api<K>, name: &str, cond: impl Condition<K>) -> Result<Option<K>, Error>
where
    K: Clone + Debug + Send + DeserializeOwned + Resource + 'static,
{
    // Skip updates until the condition is satisfied.
    let mut stream = pin!(watch_object(api, name).try_skip_while(|obj| {
        let matches = cond.matches_object(obj.as_ref());
        future::ready(Ok(!matches))
    }));

    // Then take the first update that satisfies the condition.
    let obj = stream
        .try_next()
        .await
        .map_err(Error::ProbeFailed)?
        .expect("stream must not terminate");
    Ok(obj)
}

/// A trait for condition functions to be used by [`await_condition`]
///
/// Note that this is auto-implemented for functions of type `fn(Option<&K>) -> bool`.
///
/// # Usage
///
/// ```
/// use kube::runtime::wait::Condition;
/// use k8s_openapi::api::core::v1::Pod;
/// fn my_custom_condition(my_cond: &str) -> impl Condition<Pod> + '_ {
///     move |obj: Option<&Pod>| {
///         if let Some(pod) = &obj {
///             if let Some(status) = &pod.status {
///                 if let Some(conds) = &status.conditions {
///                     if let Some(pcond) = conds.iter().find(|c| c.type_ == my_cond) {
///                         return pcond.status == "True";
///                     }
///                 }
///             }
///         }
///         false
///     }
/// }
/// ```
pub trait Condition<K> {
    fn matches_object(&self, obj: Option<&K>) -> bool;

    /// Returns a `Condition` that holds if `self` does not
    ///
    /// # Usage
    ///
    /// ```
    /// # use kube_runtime::wait::Condition;
    /// let condition: fn(Option<&()>) -> bool = |_| true;
    /// assert!(condition.matches_object(None));
    /// assert!(!condition.not().matches_object(None));
    /// ```
    fn not(self) -> conditions::Not<Self>
    where
        Self: Sized,
    {
        conditions::Not(self)
    }

    /// Returns a `Condition` that holds if `self` and `other` both do
    ///
    /// # Usage
    ///
    /// ```
    /// # use kube_runtime::wait::Condition;
    /// let cond_false: fn(Option<&()>) -> bool = |_| false;
    /// let cond_true: fn(Option<&()>) -> bool = |_| true;
    /// assert!(!cond_false.and(cond_false).matches_object(None));
    /// assert!(!cond_false.and(cond_true).matches_object(None));
    /// assert!(!cond_true.and(cond_false).matches_object(None));
    /// assert!(cond_true.and(cond_true).matches_object(None));
    /// ```
    fn and<Other: Condition<K>>(self, other: Other) -> conditions::And<Self, Other>
    where
        Self: Sized,
    {
        conditions::And(self, other)
    }

    /// Returns a `Condition` that holds if either `self` or `other` does
    ///
    /// # Usage
    ///
    /// ```
    /// # use kube_runtime::wait::Condition;
    /// let cond_false: fn(Option<&()>) -> bool = |_| false;
    /// let cond_true: fn(Option<&()>) -> bool = |_| true;
    /// assert!(!cond_false.or(cond_false).matches_object(None));
    /// assert!(cond_false.or(cond_true).matches_object(None));
    /// assert!(cond_true.or(cond_false).matches_object(None));
    /// assert!(cond_true.or(cond_true).matches_object(None));
    /// ```
    fn or<Other: Condition<K>>(self, other: Other) -> conditions::Or<Self, Other>
    where
        Self: Sized,
    {
        conditions::Or(self, other)
    }
}

impl<K, F: Fn(Option<&K>) -> bool> Condition<K> for F {
    fn matches_object(&self, obj: Option<&K>) -> bool {
        (self)(obj)
    }
}

/// Common conditions to wait for
pub mod conditions {
    pub use super::Condition;
    use k8s_openapi::{
        api::{
            apps::v1::Deployment,
            batch::v1::Job,
            core::v1::{Pod, Service},
            networking::v1::Ingress,
        },
        apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
    };
    use kube_client::Resource;

    /// An await condition that returns `true` once the object has been deleted.
    ///
    /// An object is considered to be deleted if the object can no longer be found, or if its
    /// [`uid`](kube_client::api::ObjectMeta#structfield.uid) changes. This means that an object is considered to be deleted even if we miss
    /// the deletion event and the object is recreated in the meantime.
    #[must_use]
    pub fn is_deleted<K: Resource>(uid: &str) -> impl Condition<K> + '_ {
        move |obj: Option<&K>| {
            // NB: Object not found implies success.
            obj.is_none_or(
                // Object is found, but a changed uid would mean that it was deleted and recreated
                |obj| obj.meta().uid.as_deref() != Some(uid),
            )
        }
    }

    /// An await condition for `CustomResourceDefinition` that returns `true` once it has been accepted and established
    ///
    /// Note that this condition only guarantees you that you can use `Api<CustomResourceDefinition>` when it is ready.
    /// It usually takes extra time for Discovery to notice the custom resource, and there is no condition for this.
    #[must_use]
    pub fn is_crd_established() -> impl Condition<CustomResourceDefinition> {
        |obj: Option<&CustomResourceDefinition>| {
            if let Some(o) = obj {
                if let Some(s) = &o.status {
                    if let Some(conds) = &s.conditions {
                        if let Some(pcond) = conds.iter().find(|c| c.type_ == "Established") {
                            return pcond.status == "True";
                        }
                    }
                }
            }
            false
        }
    }

    /// An await condition for `Pod` that returns `true` once it is running
    #[must_use]
    pub fn is_pod_running() -> impl Condition<Pod> {
        |obj: Option<&Pod>| {
            if let Some(pod) = &obj {
                if let Some(status) = &pod.status {
                    if let Some(phase) = &status.phase {
                        return phase == "Running";
                    }
                }
            }
            false
        }
    }

    /// An await condition for `Job` that returns `true` once it is completed
    #[must_use]
    pub fn is_job_completed() -> impl Condition<Job> {
        |obj: Option<&Job>| {
            if let Some(job) = &obj {
                if let Some(s) = &job.status {
                    if let Some(conds) = &s.conditions {
                        if let Some(pcond) = conds.iter().find(|c| c.type_ == "Complete") {
                            return pcond.status == "True";
                        }
                    }
                }
            }
            false
        }
    }

    /// An await condition for `Deployment` that returns `true` once the latest deployment has completed
    ///
    /// This looks for the condition that Kubernetes sets for completed deployments:
    /// <https://kubernetes.io/docs/concepts/workloads/controllers/deployment/#complete-deployment>
    #[must_use]
    pub fn is_deployment_completed() -> impl Condition<Deployment> {
        |obj: Option<&Deployment>| {
            if let Some(depl) = &obj {
                if let Some(s) = &depl.status {
                    if let Some(conds) = &s.conditions {
                        if let Some(dcond) = conds.iter().find(|c| {
                            c.type_ == "Progressing" && c.reason == Some("NewReplicaSetAvailable".to_string())
                        }) {
                            return dcond.status == "True";
                        }
                    }
                }
            }
            false
        }
    }

    /// An await condition for `Service`s of type `LoadBalancer` that returns `true` once the backing load balancer has an external IP or hostname
    #[must_use]
    pub fn is_service_loadbalancer_provisioned() -> impl Condition<Service> {
        |obj: Option<&Service>| {
            if let Some(svc) = &obj {
                // ignore services that are not type LoadBalancer (return true immediately)
                if let Some(spec) = &svc.spec {
                    if spec.type_ != Some("LoadBalancer".to_string()) {
                        return true;
                    }
                    // carry on if this is a LoadBalancer service
                    if let Some(s) = &svc.status {
                        if let Some(lbs) = &s.load_balancer {
                            if let Some(ings) = &lbs.ingress {
                                return ings.iter().all(|ip| ip.ip.is_some() || ip.hostname.is_some());
                            }
                        }
                    }
                }
            }
            false
        }
    }

    /// An await condition for `Ingress` that returns `true` once the backing load balancer has an external IP or hostname
    #[must_use]
    pub fn is_ingress_provisioned() -> impl Condition<Ingress> {
        |obj: Option<&Ingress>| {
            if let Some(ing) = &obj {
                if let Some(s) = &ing.status {
                    if let Some(lbs) = &s.load_balancer {
                        if let Some(ings) = &lbs.ingress {
                            return ings.iter().all(|ip| ip.ip.is_some() || ip.hostname.is_some());
                        }
                    }
                }
            }
            false
        }
    }

    /// See [`Condition::not`]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Not<A>(pub(super) A);
    impl<A: Condition<K>, K> Condition<K> for Not<A> {
        fn matches_object(&self, obj: Option<&K>) -> bool {
            !self.0.matches_object(obj)
        }
    }

    /// See [`Condition::and`]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct And<A, B>(pub(super) A, pub(super) B);
    impl<A, B, K> Condition<K> for And<A, B>
    where
        A: Condition<K>,
        B: Condition<K>,
    {
        fn matches_object(&self, obj: Option<&K>) -> bool {
            self.0.matches_object(obj) && self.1.matches_object(obj)
        }
    }

    /// See [`Condition::or`]
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct Or<A, B>(pub(super) A, pub(super) B);
    impl<A, B, K> Condition<K> for Or<A, B>
    where
        A: Condition<K>,
        B: Condition<K>,
    {
        fn matches_object(&self, obj: Option<&K>) -> bool {
            self.0.matches_object(obj) || self.1.matches_object(obj)
        }
    }

    mod tests {
        #[test]
        /// pass when CRD is established
        fn crd_established_ok() {
            use super::{is_crd_established, Condition};

            let crd = r#"
                apiVersion: apiextensions.k8s.io/v1
                kind: CustomResourceDefinition
                metadata:
                  name: testthings.kube.rs
                spec:
                  group: kube.rs
                  names:
                    categories: []
                    kind: TestThing
                    plural: testthings
                    shortNames: []
                    singular: testthing
                  scope: Namespaced
                  versions:
                    - additionalPrinterColumns: []
                      name: v1
                      schema:
                        openAPIV3Schema:
                          type: object
                          x-kubernetes-preserve-unknown-fields: true
                      served: true
                      storage: true
                status:
                  acceptedNames:
                    kind: TestThing
                    listKind: TestThingList
                    plural: testthings
                    singular: testthing
                  conditions:
                    - lastTransitionTime: "2025-03-06T03:10:03Z"
                      message: no conflicts found
                      reason: NoConflicts
                      status: "True"
                      type: NamesAccepted
                    - lastTransitionTime: "2025-03-06T03:10:03Z"
                      message: the initial names have been accepted
                      reason: InitialNamesAccepted
                      status: "True"
                      type: Established
                storedVersions:
                  - v1
            "#;

            let c = serde_yaml::from_str(crd).unwrap();
            assert!(is_crd_established().matches_object(Some(&c)))
        }

        #[test]
        /// fail when CRD is not yet ready
        fn crd_established_fail() {
            use super::{is_crd_established, Condition};

            let crd = r#"
                apiVersion: apiextensions.k8s.io/v1
                kind: CustomResourceDefinition
                metadata:
                  name: testthings.kube.rs
                spec:
                  group: kube.rs
                  names:
                    categories: []
                    kind: TestThing
                    plural: testthings
                    shortNames: []
                    singular: testthing
                  scope: Namespaced
                  versions:
                    - additionalPrinterColumns: []
                      name: v1
                      schema:
                        openAPIV3Schema:
                          type: object
                          x-kubernetes-preserve-unknown-fields: true
                      served: true
                      storage: true
                status:
                  acceptedNames:
                    kind: TestThing
                    listKind: TestThingList
                    plural: testthings
                    singular: testthing
                  conditions:
                    - lastTransitionTime: "2025-03-06T03:10:03Z"
                      message: no conflicts found
                      reason: NoConflicts
                      status: "True"
                      type: NamesAccepted
                    - lastTransitionTime: "2025-03-06T03:10:03Z"
                      message: the initial names have been accepted
                      reason: InitialNamesAccepted
                      status: "False"
                      type: Established
                storedVersions:
                  - v1
            "#;

            let c = serde_yaml::from_str(crd).unwrap();
            assert!(!is_crd_established().matches_object(Some(&c)))
        }

        #[test]
        /// fail when CRD does not exist
        fn crd_established_missing() {
            use super::{is_crd_established, Condition};

            assert!(!is_crd_established().matches_object(None))
        }

        #[test]
        /// pass when pod is running
        fn pod_running_ok() {
            use super::{is_pod_running, Condition};

            let pod = r#"
                apiVersion: v1
                kind: Pod
                metadata:
                  namespace: default
                  name: testpod
                spec:
                  containers:
                    - name: testcontainer
                      image: alpine
                      command: [ sleep ]
                      args: [ "100000" ]
                status:
                  conditions:
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:53:07Z"
                      status: "True"
                      type: PodReadyToStartContainers
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:52:58Z"
                      status: "True"
                      type: Initialized
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:53:24Z"
                      status: "True"
                      type: Ready
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:53:24Z"
                      status: "True"
                      type: ContainersReady
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:52:58Z"
                      status: "True"
                      type: PodScheduled
                  containerStatuses:
                    - containerID: containerd://598323380ae59d60c1ab98f9091c94659137a976d52136a8083775d47fea5875
                      image: docker.io/library/alpine:latest
                      imageID: docker.io/library/alpine@sha256:a8560b36e8b8210634f77d9f7f9efd7ffa463e380b75e2e74aff4511df3ef88c
                      lastState: {}
                      name: testcontainer
                      ready: true
                      restartCount: 0
                      started: true
                      state:
                        running:
                          startedAt: "2025-03-06T03:59:20Z"
                  phase: Running
                  qosClass: Burstable
            "#;

            let p = serde_yaml::from_str(pod).unwrap();
            assert!(is_pod_running().matches_object(Some(&p)))
        }

        #[test]
        /// fail if pod is unschedulable
        fn pod_running_unschedulable() {
            use super::{is_pod_running, Condition};

            let pod = r#"
                apiVersion: v1
                kind: Pod
                metadata:
                  namespace: default
                  name: testpod
                spec:
                  containers:
                    - name: testcontainer
                      image: alpine
                      command: [ sleep ]
                      args: [ "100000" ]
                status:
                  conditions:
                    - lastProbeTime: null
                      lastTransitionTime: "2025-03-06T03:52:25Z"
                      message: '0/1 nodes are available: 1 node(s) were unschedulable. preemption: 0/1
                      nodes are available: 1 Preemption is not helpful for scheduling.'
                      reason: Unschedulable
                      status: "False"
                      type: PodScheduled
                  phase: Pending
                  qosClass: Burstable
            "#;

            let p = serde_yaml::from_str(pod).unwrap();
            assert!(!is_pod_running().matches_object(Some(&p)))
        }

        #[test]
        /// fail if pod does not exist
        fn pod_running_missing() {
            use super::{is_pod_running, Condition};

            assert!(!is_pod_running().matches_object(None))
        }

        #[test]
        /// pass if job completed
        fn job_completed_ok() {
            use super::{is_job_completed, Condition};

            let job = r#"
                apiVersion: batch/v1
                kind: Job
                metadata:
                  name: pi
                  namespace: default
                spec:
                  template:
                    spec:
                      containers:
                      - name: pi
                        command:
                        - perl
                        - -Mbignum=bpi
                        - -wle
                        - print bpi(2000)
                        image: perl:5.34.0
                        imagePullPolicy: IfNotPresent
                status:
                  completionTime: "2025-03-06T05:27:56Z"
                  conditions:
                  - lastProbeTime: "2025-03-06T05:27:56Z"
                    lastTransitionTime: "2025-03-06T05:27:56Z"
                    message: Reached expected number of succeeded pods
                    reason: CompletionsReached
                    status: "True"
                    type: SuccessCriteriaMet
                  - lastProbeTime: "2025-03-06T05:27:56Z"
                    lastTransitionTime: "2025-03-06T05:27:56Z"
                    message: Reached expected number of succeeded pods
                    reason: CompletionsReached
                    status: "True"
                    type: Complete
                  ready: 0
                  startTime: "2025-03-06T05:27:27Z"
                  succeeded: 1
                  terminating: 0
                  uncountedTerminatedPods: {}
            "#;

            let j = serde_yaml::from_str(job).unwrap();
            assert!(is_job_completed().matches_object(Some(&j)))
        }

        #[test]
        /// fail if job is still in progress
        fn job_completed_running() {
            use super::{is_job_completed, Condition};

            let job = r#"
                apiVersion: batch/v1
                kind: Job
                metadata:
                  name: pi
                  namespace: default
                spec:
                  backoffLimit: 4
                  completionMode: NonIndexed
                  completions: 1
                  manualSelector: false
                  parallelism: 1
                  template:
                    spec:
                      containers:
                      - name: pi
                        command:
                        - perl
                        - -Mbignum=bpi
                        - -wle
                        - print bpi(2000)
                        image: perl:5.34.0
                        imagePullPolicy: IfNotPresent
                status:
                  active: 1
                  ready: 0
                  startTime: "2025-03-06T05:27:27Z"
                  terminating: 0
                  uncountedTerminatedPods: {}
            "#;

            let j = serde_yaml::from_str(job).unwrap();
            assert!(!is_job_completed().matches_object(Some(&j)))
        }

        #[test]
        /// fail if job does not exist
        fn job_completed_missing() {
            use super::{is_job_completed, Condition};

            assert!(!is_job_completed().matches_object(None))
        }

        #[test]
        /// pass when deployment has been fully rolled out
        fn deployment_completed_ok() {
            use super::{is_deployment_completed, Condition};

            let depl = r#"
                apiVersion: apps/v1
                kind: Deployment
                metadata:
                  name: testapp
                  namespace: default
                spec:
                  progressDeadlineSeconds: 600
                  replicas: 3
                  revisionHistoryLimit: 10
                  selector:
                    matchLabels:
                      app: test
                  strategy:
                    rollingUpdate:
                      maxSurge: 25%
                      maxUnavailable: 25%
                    type: RollingUpdate
                  template:
                    metadata:
                      creationTimestamp: null
                      labels:
                        app: test
                    spec:
                      containers:
                      - image: postgres
                        imagePullPolicy: Always
                        name: postgres
                        ports:
                        - containerPort: 5432
                          protocol: TCP
                        env:
                        - name: POSTGRES_PASSWORD
                          value: foobar
                status:
                  availableReplicas: 3
                  conditions:
                  - lastTransitionTime: "2025-03-06T06:06:57Z"
                    lastUpdateTime: "2025-03-06T06:06:57Z"
                    message: Deployment has minimum availability.
                    reason: MinimumReplicasAvailable
                    status: "True"
                    type: Available
                  - lastTransitionTime: "2025-03-06T06:03:20Z"
                    lastUpdateTime: "2025-03-06T06:06:57Z"
                    message: ReplicaSet "testapp-7fcd4b58c9" has successfully progressed.
                    reason: NewReplicaSetAvailable
                    status: "True"
                    type: Progressing
                  observedGeneration: 2
                  readyReplicas: 3
                  replicas: 3
                  updatedReplicas: 3
            "#;

            let d = serde_yaml::from_str(depl).unwrap();
            assert!(is_deployment_completed().matches_object(Some(&d)))
        }

        #[test]
        /// fail if deployment update is still rolling out
        fn deployment_completed_pending() {
            use super::{is_deployment_completed, Condition};

            let depl = r#"
                apiVersion: apps/v1
                kind: Deployment
                metadata:
                  name: testapp
                  namespace: default
                spec:
                  progressDeadlineSeconds: 600
                  replicas: 3
                  revisionHistoryLimit: 10
                  selector:
                    matchLabels:
                      app: test
                  strategy:
                    rollingUpdate:
                      maxSurge: 25%
                      maxUnavailable: 25%
                    type: RollingUpdate
                  template:
                    metadata:
                      creationTimestamp: null
                      labels:
                        app: test
                    spec:
                      containers:
                      - image: postgres
                        imagePullPolicy: Always
                        name: postgres
                        ports:
                        - containerPort: 5432
                          protocol: TCP
                        env:
                        - name: POSTGRES_PASSWORD
                          value: foobar
                status:
                  conditions:
                  - lastTransitionTime: "2025-03-06T06:03:20Z"
                    lastUpdateTime: "2025-03-06T06:03:20Z"
                    message: Deployment does not have minimum availability.
                    reason: MinimumReplicasUnavailable
                    status: "False"
                    type: Available
                  - lastTransitionTime: "2025-03-06T06:03:20Z"
                    lastUpdateTime: "2025-03-06T06:03:20Z"
                    message: ReplicaSet "testapp-77789cd7d4" is progressing.
                    reason: ReplicaSetUpdated
                    status: "True"
                    type: Progressing
                  observedGeneration: 1
                  replicas: 3
                  unavailableReplicas: 3
                  updatedReplicas: 3
            "#;

            let d = serde_yaml::from_str(depl).unwrap();
            assert!(!is_deployment_completed().matches_object(Some(&d)))
        }

        #[test]
        /// fail if deployment does not exist
        fn deployment_completed_missing() {
            use super::{is_deployment_completed, Condition};

            assert!(!is_deployment_completed().matches_object(None))
        }

        #[test]
        /// pass if loadbalancer service has recieved a loadbalancer IP
        fn service_lb_provisioned_ok_ip() {
            use super::{is_service_loadbalancer_provisioned, Condition};

            let service = r"
                apiVersion: v1
                kind: Service
                metadata:
                  name: test
                spec:
                  selector:
                    app.kubernetes.io/name: test
                  type: LoadBalancer
                  ports:
                    - protocol: TCP
                      port: 80
                      targetPort: 9376
                  clusterIP: 10.0.171.239
                status:
                  loadBalancer:
                    ingress:
                      - ip: 192.0.2.127
            ";

            let s = serde_yaml::from_str(service).unwrap();
            assert!(is_service_loadbalancer_provisioned().matches_object(Some(&s)))
        }

        #[test]
        /// pass if loadbalancer service has recieved a loadbalancer hostname
        fn service_lb_provisioned_ok_hostname() {
            use super::{is_service_loadbalancer_provisioned, Condition};

            let service = r"
                apiVersion: v1
                kind: Service
                metadata:
                  name: test
                spec:
                  selector:
                    app.kubernetes.io/name: test
                  type: LoadBalancer
                  ports:
                    - protocol: TCP
                      port: 80
                      targetPort: 9376
                  clusterIP: 10.0.171.239
                status:
                  loadBalancer:
                    ingress:
                      - hostname: example.exposed.service
            ";

            let s = serde_yaml::from_str(service).unwrap();
            assert!(is_service_loadbalancer_provisioned().matches_object(Some(&s)))
        }

        #[test]
        /// fail if loadbalancer service is still waiting for a LB
        fn service_lb_provisioned_pending() {
            use super::{is_service_loadbalancer_provisioned, Condition};

            let service = r"
                apiVersion: v1
                kind: Service
                metadata:
                  name: test
                spec:
                  selector:
                    app.kubernetes.io/name: test
                  type: LoadBalancer
                  ports:
                    - protocol: TCP
                      port: 80
                      targetPort: 9376
                  clusterIP: 10.0.171.239
                status:
                  loadBalancer: {}
            ";

            let s = serde_yaml::from_str(service).unwrap();
            assert!(!is_service_loadbalancer_provisioned().matches_object(Some(&s)))
        }

        #[test]
        /// pass if service is not a loadbalancer
        fn service_lb_provisioned_not_loadbalancer() {
            use super::{is_service_loadbalancer_provisioned, Condition};

            let service = r"
                apiVersion: v1
                kind: Service
                metadata:
                  name: test
                spec:
                  selector:
                    app.kubernetes.io/name: test
                  type: ClusterIP
                  ports:
                    - protocol: TCP
                      port: 80
                      targetPort: 9376
                status:
                  loadBalancer: {}
            ";

            let s = serde_yaml::from_str(service).unwrap();
            assert!(is_service_loadbalancer_provisioned().matches_object(Some(&s)))
        }

        #[test]
        /// fail if service does not exist
        fn service_lb_provisioned_missing() {
            use super::{is_service_loadbalancer_provisioned, Condition};

            assert!(!is_service_loadbalancer_provisioned().matches_object(None))
        }

        #[test]
        /// pass when ingress has recieved a loadbalancer IP
        fn ingress_provisioned_ok_ip() {
            use super::{is_ingress_provisioned, Condition};

            let ingress = r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  name: test
                  namespace: default
                  resourceVersion: "1401"
                  uid: d653ee4d-0adb-40d9-b03c-7f84f35d4a67
                spec:
                  ingressClassName: nginx
                  rules:
                  - host: httpbin.local
                    http:
                      paths:
                      - path: /
                        backend:
                          service:
                            name: httpbin
                            port:
                              number: 80
                status:
                  loadBalancer:
                    ingress:
                      - ip: 10.89.7.3
            "#;

            let i = serde_yaml::from_str(ingress).unwrap();
            assert!(is_ingress_provisioned().matches_object(Some(&i)))
        }

        #[test]
        /// pass when ingress has recieved a loadbalancer hostname
        fn ingress_provisioned_ok_hostname() {
            use super::{is_ingress_provisioned, Condition};

            let ingress = r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  name: test
                  namespace: default
                  resourceVersion: "1401"
                  uid: d653ee4d-0adb-40d9-b03c-7f84f35d4a67
                spec:
                  ingressClassName: nginx
                  rules:
                  - host: httpbin.local
                    http:
                      paths:
                      - path: /
                        backend:
                          service:
                            name: httpbin
                            port:
                              number: 80
                status:
                  loadBalancer:
                    ingress:
                      - hostname: example.exposed.service
            "#;

            let i = serde_yaml::from_str(ingress).unwrap();
            assert!(is_ingress_provisioned().matches_object(Some(&i)))
        }

        #[test]
        /// fail if ingress is still waiting for a LB
        fn ingress_provisioned_pending() {
            use super::{is_ingress_provisioned, Condition};

            let ingress = r#"
                apiVersion: networking.k8s.io/v1
                kind: Ingress
                metadata:
                  name: test
                  namespace: default
                  resourceVersion: "1401"
                  uid: d653ee4d-0adb-40d9-b03c-7f84f35d4a67
                spec:
                  ingressClassName: nginx
                  rules:
                  - host: httpbin.local
                    http:
                      paths:
                      - path: /
                        backend:
                          service:
                            name: httpbin
                            port:
                              number: 80
                status:
                  loadBalancer: {}
            "#;

            let i = serde_yaml::from_str(ingress).unwrap();
            assert!(!is_ingress_provisioned().matches_object(Some(&i)))
        }

        #[test]
        /// fail if ingress does not exist
        fn ingress_provisioned_missing() {
            use super::{is_ingress_provisioned, Condition};

            assert!(!is_ingress_provisioned().matches_object(None))
        }
    }
}

/// Utilities for deleting objects
pub mod delete {
    use super::{await_condition, conditions};
    use kube_client::{api::DeleteParams, Api, Resource};
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;
    use thiserror::Error;

    #[derive(Debug, Error)]
    pub enum Error {
        #[error("deleted object has no UID to wait for")]
        NoUid,
        #[error("failed to delete object: {0}")]
        Delete(#[source] kube_client::Error),
        #[error("failed to wait for object to be deleted: {0}")]
        Await(#[source] super::Error),
    }

    /// Delete an object, and wait for it to be removed from the Kubernetes API (including waiting for all finalizers to unregister themselves).
    ///
    /// # Errors
    ///
    /// Returns an [`Error`](enum@super::Error) if the object was unable to be deleted, or if the wait was interrupted.
    #[allow(clippy::module_name_repetitions)]
    pub async fn delete_and_finalize<K: Clone + Debug + Send + DeserializeOwned + Resource + 'static>(
        api: Api<K>,
        name: &str,
        delete_params: &DeleteParams,
    ) -> Result<(), Error> {
        let deleted_obj_uid = api
            .delete(name, delete_params)
            .await
            .map_err(Error::Delete)?
            .either(
                |mut obj| obj.meta_mut().uid.take(),
                |status| status.details.map(|details| details.uid),
            )
            .ok_or(Error::NoUid)?;
        await_condition(api, name, conditions::is_deleted(&deleted_obj_uid))
            .await
            .map_err(Error::Await)?;
        Ok(())
    }
}
