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
            obj.map_or(
                // Object is not found, success!
                true,
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
