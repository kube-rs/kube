use futures::TryStreamExt;
use kube_client::{Api, Resource};
use serde::de::DeserializeOwned;
use snafu::{futures::TryStreamExt as _, Snafu};
use std::fmt::Debug;

use crate::watcher::{self, watch_object};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("failed to probe for whether the condition is fulfilled yet: {}", source))]
    ProbeFailed {
        #[snafu(backtrace)]
        source: watcher::Error,
    },
}

/// Watch an object, and Wait for some condition `cond` to return `true`.
///
/// `cond` is passed `Some` if the object is found, otherwise `None`.
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
pub async fn await_condition<K>(api: Api<K>, name: &str, cond: impl Condition<K>) -> Result<(), Error>
where
    K: Clone + Debug + Send + DeserializeOwned + Resource + 'static,
{
    watch_object(api, name)
        .context(ProbeFailed)
        .try_take_while(|obj| {
            let result = !cond.matches_object(obj.as_ref());
            async move { Ok(result) }
        })
        .try_for_each(|_| async { Ok(()) })
        .await
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
}

impl<K, F: Fn(Option<&K>) -> bool> Condition<K> for F {
    fn matches_object(&self, obj: Option<&K>) -> bool {
        (self)(obj)
    }
}

/// Common conditions to wait for
pub mod conditions {
    pub use super::Condition;
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    use kube_client::Resource;

    /// An await condition that returns `true` once the object has been deleted.
    ///
    /// An object is considered to be deleted if the object can no longer be found, or if its
    /// [`uid`](kube_client::api::ObjectMeta#structfield.uid) changes. This means that an object is considered to be deleted even if we miss
    /// the deletion event and the object is recreated in the meantime.
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
}

/// Utilities for deleting objects
pub mod delete {
    use super::{await_condition, conditions};
    use kube_client::{api::DeleteParams, Api, Resource};
    use serde::de::DeserializeOwned;
    use snafu::{OptionExt, ResultExt, Snafu};
    use std::fmt::Debug;

    #[derive(Snafu, Debug)]
    pub enum Error {
        #[snafu(display("deleted object has no UID to wait for"))]
        NoUid,
        #[snafu(display("failed to delete object: {}", source))]
        Delete { source: kube_client::Error },
        #[snafu(display("failed to wait for object to be deleted: {}", source))]
        Await { source: super::Error },
    }

    /// Delete an object, and wait for it to be removed from the Kubernetes API (including waiting for all finalizers to unregister themselves).
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the object was unable to be deleted, or if the wait was interrupted.
    #[allow(clippy::module_name_repetitions)]
    pub async fn delete_and_finalize<K: Clone + Debug + Send + DeserializeOwned + Resource + 'static>(
        api: Api<K>,
        name: &str,
        delete_params: &DeleteParams,
    ) -> Result<(), Error> {
        let deleted_obj_uid = api
            .delete(name, delete_params)
            .await
            .context(Delete)?
            .either(
                |mut obj| obj.meta_mut().uid.take(),
                |status| status.details.map(|details| details.uid),
            )
            .context(NoUid)?;
        await_condition(api, name, conditions::is_deleted(&deleted_obj_uid))
            .await
            .context(Await)
    }
}
