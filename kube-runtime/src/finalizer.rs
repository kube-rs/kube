use crate::{controller::ReconcilerAction, watcher};
use futures::{pin_mut, TryFuture, TryFutureExt, TryStreamExt};
use json_patch::{AddOperation, PatchOperation, RemoveOperation, TestOperation};
use k8s_openapi::Metadata;
use kube::{
    api::{DeleteParams, ListParams, Patch, PatchParams},
    Api, Resource, ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};
use snafu::{OptionExt, ResultExt, Snafu};
use std::{error::Error as StdError, fmt::Debug};

#[derive(Debug, Snafu)]
pub enum Error<ReconcileErr>
where
    ReconcileErr: StdError + 'static,
{
    #[snafu(display("failed to apply object: {}", source))]
    ApplyFailed { source: ReconcileErr },
    #[snafu(display("failed to clean up object: {}", source))]
    CleanupFailed { source: ReconcileErr },
    #[snafu(display("failed to add finalizer: {}", source))]
    AddFinalizer { source: kube::Error },
    #[snafu(display("failed to remove finalizer: {}", source))]
    RemoveFinalizer { source: kube::Error },
    #[snafu(display("object has no name"))]
    UnnamedObject,
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

/// Reconcile an object in a way that requires cleanup before an object can be deleted. It does this by
/// managing a [`ObjectMeta::finalizers`] entry, which prevents the object from being deleted before the
/// cleanup is done.
///
/// In typical usage, if you use `finalizer` then it should be the only top-level "action"
/// in your [`applier`]/[`Controller`]'s `reconcile` function.
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
pub async fn finalizer<K, ReconcileFut>(
    api: &Api<K>,
    finalizer_name: &str,
    obj: K,
    reconcile: impl FnOnce(Event<K>) -> ReconcileFut,
) -> Result<ReconcilerAction, Error<ReconcileFut::Error>>
where
    K: Resource + Clone + DeserializeOwned + Serialize + Debug,
    ReconcileFut: TryFuture<Ok = ReconcilerAction>,
    ReconcileFut::Error: StdError + 'static,
{
    match FinalizerState::for_object(&obj, finalizer_name) {
        FinalizerState {
            finalizer_index: Some(_),
            is_deleting: false,
        } => reconcile(Event::Apply(obj))
            .into_future()
            .await
            .context(ApplyFailed),
        FinalizerState {
            finalizer_index: Some(finalizer_i),
            is_deleting: true,
        } => {
            // Cleanup reconciliation must succeed before it's safe to remove the finalizer
            let name = obj.meta().name.clone().context(UnnamedObject)?;
            let action = reconcile(Event::Cleanup(obj))
                .into_future()
                .await
                // Short-circuit, so that we keep the finalizer if cleanup fails
                .context(CleanupFailed)?;
            // Cleanup was successful, remove the finalizer so that deletion can continue
            let finalizer_path = format!("/metadata/finalizers/{}", finalizer_i);
            api.patch::<K>(
                &name,
                &PatchParams::default(),
                &Patch::Json(json_patch::Patch(vec![
                    // All finalizers run concurrently and we use an integer index
                    // `Test` ensures that we fail instead of deleting someone else's finalizer
                    // (in which case a new `Cleanup` event will be sent)
                    PatchOperation::Test(TestOperation {
                        path: finalizer_path.clone(),
                        value: finalizer_name.into(),
                    }),
                    PatchOperation::Remove(RemoveOperation { path: finalizer_path }),
                ])),
            )
            .await
            .context(RemoveFinalizer)?;
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
                        path: "/metadata/finalizers".to_string(),
                        value: serde_json::Value::Null,
                    }),
                    PatchOperation::Add(AddOperation {
                        path: "/metadata/finalizers".to_string(),
                        value: vec![finalizer_name].into(),
                    }),
                ]
            } else {
                vec![PatchOperation::Add(AddOperation {
                    path: "/metadata/finalizers/-".to_string(),
                    value: finalizer_name.into(),
                })]
            });
            api.patch::<K>(
                obj.meta().name.as_deref().context(UnnamedObject)?,
                &PatchParams::default(),
                &Patch::Json(patch),
            )
            .await
            .context(AddFinalizer)?;
            // No point applying here, since the patch will cause a new reconciliation
            Ok(ReconcilerAction { requeue_after: None })
        }
        FinalizerState {
            finalizer_index: None,
            is_deleting: true,
        } => {
            // Our work here is done
            Ok(ReconcilerAction { requeue_after: None })
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
    Apply(K),
    /// The object is being deleted, and the reconciler should remove all resources that it owns.
    ///
    /// This must be idempotent, since it may be recalled if, for example (this list is non-exhaustive):
    ///
    /// - The controller is restarted while the deletion is in progress
    /// - The reconciliation fails
    /// - Another finalizer was removed in the meantime
    /// - The grinch's heart grows a size or two
    Cleanup(K),
}

#[derive(Debug, Snafu)]
pub enum DeleteError {
    #[snafu(display("failed to delete object: {}", source))]
    DeleteFailed { source: kube::Error },
    #[snafu(display("failed to probe for whether the object is deleted yet: {}", source))]
    DeleteProbeFailed { source: watcher::Error },
}

/// Try to delete an object, and wait for it to disappear from the Kubernetes API.
/// If you do not wish to wait for the object to be deleted then you should use
/// [`Api::delete`] instead.
///
/// This means that this function will not complete until all finalizers are done.
///
/// # Errors
///
/// This function fails if the deletion or status probe failed. In particular, it requires
/// permission to use the following Kubernetes verbs:
///
/// - delete
/// - list
/// - watch
///
/// Note that the deletion will continue in the background if the probe fails.
pub async fn finalize_and_delete<K>(api: &Api<K>, name: &str, dp: &DeleteParams) -> Result<(), DeleteError>
where
    K: Clone + DeserializeOwned + Debug + Send + Resource + 'static,
{
    api.delete(name, dp).await.context(DeleteFailed)?;
    let watcher = watcher(api.clone(), ListParams {
        field_selector: Some(format!("metadata.name={}", name)),
        ..Default::default()
    });
    pin_mut!(watcher);
    while let Some(event) = watcher.try_next().await.context(DeleteProbeFailed)? {
        match event {
            // Object was deleted, we're done
            watcher::Event::Deleted(_) => break,
            // Object was deleted during desync, we're still done
            watcher::Event::Restarted(objs) if objs.is_empty() => break,
            // Object is still around, wait for a while longer :(
            watcher::Event::Restarted(_) | watcher::Event::Applied(_) => (),
        }
    }
    Ok(())
}
