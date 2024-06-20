//! Finalizer helper for [`Controller`](crate::Controller) reconcilers
use crate::controller::Action;
use futures::{TryFuture, TryFutureExt};
use json_patch::{AddOperation, PatchOperation, RemoveOperation, TestOperation};
use jsonptr::Pointer;
use kube_client::{
    api::{Patch, PatchParams},
    Api, Resource, ResourceExt,
};

use serde::{de::DeserializeOwned, Serialize};
use std::{error::Error as StdError, fmt::Debug, str::FromStr, sync::Arc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error<ReconcileErr>
where
    ReconcileErr: StdError + 'static,
{
    #[error("failed to apply object: {0}")]
    ApplyFailed(#[source] ReconcileErr),
    #[error("failed to clean up object: {0}")]
    CleanupFailed(#[source] ReconcileErr),
    #[error("failed to add finalizer: {0}")]
    AddFinalizer(#[source] kube_client::Error),
    #[error("failed to remove finalizer: {0}")]
    RemoveFinalizer(#[source] kube_client::Error),
    #[error("object has no name")]
    UnnamedObject,
    #[error("invalid finalizer")]
    InvalidFinalizer,
}

struct FinalizerState {
    finalizer_index: Option<usize>,
    is_deleting: bool,
}

impl FinalizerState {
    fn for_object<K: Resource>(obj: &K, finalizer_name: &str) -> Self {
        Self {
            finalizer_index: obj
                .finalizers()
                .iter()
                .enumerate()
                .find(|(_, fin)| *fin == finalizer_name)
                .map(|(i, _)| i),
            is_deleting: obj.meta().deletion_timestamp.is_some(),
        }
    }
}

/// Reconcile an object in a way that requires cleanup before an object can be deleted.
///
/// It does this by managing a [`ObjectMeta::finalizers`] entry,
/// which prevents the object from being deleted before the cleanup is done.
///
/// In typical usage, if you use `finalizer` then it should be the only top-level "action"
/// in your [`applier`](crate::applier)/[`Controller`](crate::Controller)'s `reconcile` function.
///
/// # Expected Flow
///
/// 1. User creates object
/// 2. Reconciler sees object
/// 3. `finalizer` adds `finalizer_name` to [`ObjectMeta::finalizers`]
/// 4. Reconciler sees updated object
/// 5. `finalizer` runs [`Event::Apply`]
/// 6. User updates object
/// 7. Reconciler sees updated object
/// 8. `finalizer` runs [`Event::Apply`]
/// 9. User deletes object
/// 10. Reconciler sees deleting object
/// 11. `finalizer` runs [`Event::Cleanup`]
/// 12. `finalizer` removes `finalizer_name` from [`ObjectMeta::finalizers`]
/// 13. Kubernetes sees that all [`ObjectMeta::finalizers`] are gone and finally deletes the object
///
/// # Guarantees
///
/// If [`Event::Apply`] is ever started then [`Event::Cleanup`] must succeed before the Kubernetes object deletion completes.
///
/// # Assumptions
///
/// `finalizer_name` must be unique among the controllers interacting with the object
///
/// [`Event::Apply`] and [`Event::Cleanup`] must both be idempotent, and tolerate being executed several times (even if previously cancelled).
///
/// [`Event::Cleanup`] must tolerate [`Event::Apply`] never having ran at all, or never having succeeded. Keep in mind that
/// even infallible `.await`s are cancellation points.
///
/// # Caveats
///
/// Object deletes will get stuck while the controller is not running, or if `cleanup` fails for some reason.
///
/// `reconcile` should take the object that the [`Event`] contains, rather than trying to reuse `obj`, since it may have been updated.
///
/// # Errors
///
/// [`Event::Apply`] and [`Event::Cleanup`] are both fallible, their errors are passed through as [`Error::ApplyFailed`]
/// and [`Error::CleanupFailed`], respectively.
///
/// In addition, adding and removing the finalizer itself may fail. In particular, this may be because of
/// network errors, lacking permissions, or because another `finalizer` was updated in the meantime on the same object.
///
/// [`ObjectMeta::finalizers`]: kube_client::api::ObjectMeta#structfield.finalizers
pub async fn finalizer<K, ReconcileFut>(
    api: &Api<K>,
    finalizer_name: &str,
    obj: Arc<K>,
    reconcile: impl FnOnce(Event<K>) -> ReconcileFut,
) -> Result<Action, Error<ReconcileFut::Error>>
where
    K: Resource + Clone + DeserializeOwned + Serialize + Debug,
    ReconcileFut: TryFuture<Ok = Action>,
    ReconcileFut::Error: StdError + 'static,
{
    match FinalizerState::for_object(&*obj, finalizer_name) {
        FinalizerState {
            finalizer_index: Some(_),
            is_deleting: false,
        } => reconcile(Event::Apply(obj))
            .into_future()
            .await
            .map_err(Error::ApplyFailed),
        FinalizerState {
            finalizer_index: Some(finalizer_i),
            is_deleting: true,
        } => {
            // Cleanup reconciliation must succeed before it's safe to remove the finalizer
            let name = obj.meta().name.clone().ok_or(Error::UnnamedObject)?;
            let action = reconcile(Event::Cleanup(obj))
                .into_future()
                .await
                // Short-circuit, so that we keep the finalizer if cleanup fails
                .map_err(Error::CleanupFailed)?;
            // Cleanup was successful, remove the finalizer so that deletion can continue
            let finalizer_path = format!("/metadata/finalizers/{finalizer_i}");
            api.patch::<K>(
                &name,
                &PatchParams::default(),
                &Patch::Json(json_patch::Patch(vec![
                    // All finalizers run concurrently and we use an integer index
                    // `Test` ensures that we fail instead of deleting someone else's finalizer
                    // (in which case a new `Cleanup` event will be sent)
                    PatchOperation::Test(TestOperation {
                        path: Pointer::from_str(finalizer_path.as_str())
                            .map_err(|_err| Error::InvalidFinalizer)?,
                        value: finalizer_name.into(),
                    }),
                    PatchOperation::Remove(RemoveOperation {
                        path: Pointer::from_str(finalizer_path.as_str())
                            .map_err(|_err| Error::InvalidFinalizer)?,
                    }),
                ])),
            )
            .await
            .map_err(Error::RemoveFinalizer)?;
            Ok(action)
        }
        FinalizerState {
            finalizer_index: None,
            is_deleting: false,
        } => {
            // Finalizer must be added before it's safe to run an `Apply` reconciliation
            let patch = json_patch::Patch(if obj.finalizers().is_empty() {
                vec![
                    PatchOperation::Test(TestOperation {
                        path: Pointer::from_str("/metadata/finalizers")
                            .map_err(|_err| Error::InvalidFinalizer)?,
                        value: serde_json::Value::Null,
                    }),
                    PatchOperation::Add(AddOperation {
                        path: Pointer::from_str("/metadata/finalizers")
                            .map_err(|_err| Error::InvalidFinalizer)?,
                        value: vec![finalizer_name].into(),
                    }),
                ]
            } else {
                vec![
                    // Kubernetes doesn't automatically deduplicate finalizers (see
                    // https://github.com/kube-rs/kube/issues/964#issuecomment-1197311254),
                    // so we need to fail and retry if anyone else has added the finalizer in the meantime
                    PatchOperation::Test(TestOperation {
                        path: Pointer::from_str("/metadata/finalizers")
                            .map_err(|_err| Error::InvalidFinalizer)?,
                        value: obj.finalizers().into(),
                    }),
                    PatchOperation::Add(AddOperation {
                        path: Pointer::from_str("/metadata/finalizers/-")
                            .map_err(|_err| Error::InvalidFinalizer)?,
                        value: finalizer_name.into(),
                    }),
                ]
            });
            api.patch::<K>(
                obj.meta().name.as_deref().ok_or(Error::UnnamedObject)?,
                &PatchParams::default(),
                &Patch::Json(patch),
            )
            .await
            .map_err(Error::AddFinalizer)?;
            // No point applying here, since the patch will cause a new reconciliation
            Ok(Action::await_change())
        }
        FinalizerState {
            finalizer_index: None,
            is_deleting: true,
        } => {
            // Our work here is done
            Ok(Action::await_change())
        }
    }
}

/// A representation of an action that should be taken by a reconciler.
pub enum Event<K> {
    /// The reconciler should ensure that the actual state matches the state desired in the object.
    ///
    /// This must be idempotent, since it may be recalled if, for example (this list is non-exhaustive):
    ///
    /// - The controller is restarted
    /// - The object is updated
    /// - The reconciliation fails
    /// - The grinch attacks
    Apply(Arc<K>),
    /// The object is being deleted, and the reconciler should remove all resources that it owns.
    ///
    /// This must be idempotent, since it may be recalled if, for example (this list is non-exhaustive):
    ///
    /// - The controller is restarted while the deletion is in progress
    /// - The reconciliation fails
    /// - Another finalizer was removed in the meantime
    /// - The grinch's heart grows a size or two
    Cleanup(Arc<K>),
}
