use crate::controller::ReconcilerAction;
use futures::{TryFuture, TryFutureExt};
use json_patch::{AddOperation, PatchOperation, RemoveOperation, TestOperation};
use k8s_openapi::Metadata;
use kube::{
    api::{ObjectMeta, Patch, PatchParams},
    Api,
};
use serde::{de::DeserializeOwned, Serialize};
use snafu::{ResultExt, Snafu};
use std::{error::Error as StdError, fmt::Debug};

#[derive(Debug, Snafu)]
pub enum Error<ApplyErr, CleanupErr>
where
    ApplyErr: StdError + 'static,
    CleanupErr: StdError + 'static,
{
    #[snafu(display("failed to apply object: {}", source))]
    ApplyFailed { source: ApplyErr },
    #[snafu(display("failed to clean up object: {}", source))]
    CleanupFailed { source: CleanupErr },
    #[snafu(display("failed to add finalizer: {}", source))]
    AddFinalizer { source: kube::Error },
    #[snafu(display("failed to remove finalizer: {}", source))]
    RemoveFinalizer { source: kube::Error },
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
/// 5. `finalizer` runs `apply`
/// 6. User updates object
/// 7. Reconciler sees updated object
/// 8. `finalizer` runs `apply`
/// 9. User deletes object
/// 10. Reconciler sees deleting object
/// 11. `finalizer` runs `cleanup`
/// 12. `finalizer` removes `finalizer_name` from [`ObjectMeta::finalizers`]
/// 13. Kubernetes sees that all [`ObjectMeta::finalizers`] are gone and finally deletes the object
///
/// # Guarantees
///
/// If `apply` is ever started then `cleanup` must succeed before the Kubernetes object deletion completes.
///
/// # Assumptions
///
/// `finalizer_name` must be unique among the controllers interacting with the object
///
/// `apply` and `cleanup` must both be idempotent, and tolerate being executed several times (even if previously cancelled).
///
/// `cleanup` must tolerate `apply` never having ran at all, or never having succeeded. Keep in mind that
/// even infallible `.await`s are cancellation points.
///
/// # Caveats
///
/// Object deletes will get stuck while the controller is not running, or if `cleanup` fails for some reason.
///
/// # Errors
///
/// `apply` and `cleanup` are both fallible, their errors are passed through as [`Error::ApplyFailed`]
/// and [`Error::CleanupFailed`], respectively.
///
/// In addition, adding and removing the finalizer itself may fail. In particular, this may be because of
/// network errors, or because another `finalizer` was updated in the meantime on the same object.
pub async fn finalizer<K, ApplyFut, CleanupFut>(
    api: &Api<K>,
    finalizer_name: &str,
    obj: K,
    apply: impl FnOnce(K) -> ApplyFut,
    cleanup: impl FnOnce(K) -> CleanupFut,
) -> Result<ReconcilerAction, Error<ApplyFut::Error, CleanupFut::Error>>
where
    K: Metadata<Ty = ObjectMeta> + Clone + DeserializeOwned + Serialize + Debug,
    ApplyFut: TryFuture<Ok = ReconcilerAction>,
    ApplyFut::Error: StdError + 'static,
    CleanupFut: TryFuture<Ok = ()>,
    CleanupFut::Error: StdError + 'static,
{
    if let Some((finalizer_i, _)) = obj
        .metadata()
        .finalizers
        .as_ref()
        .and_then(|fins| fins.iter().enumerate().find(|(_, fin)| *fin == finalizer_name))
    {
        if obj.metadata().deletion_timestamp.is_none() {
            apply(obj).into_future().await.context(ApplyFailed)
        } else {
            let name = obj.metadata().name.clone().unwrap();
            cleanup(obj).into_future().await.context(CleanupFailed)?;
            let finalizer_path = format!("/metadata/finalizers/{}", finalizer_i);
            api.patch::<K>(
                &name,
                &PatchParams::default(),
                &Patch::Json(json_patch::Patch(vec![
                    PatchOperation::Test(TestOperation {
                        path: finalizer_path.clone(),
                        value: finalizer_name.into(),
                    }),
                    PatchOperation::Remove(RemoveOperation { path: finalizer_path }),
                ])),
            )
            .await
            .context(RemoveFinalizer)?;
            Ok(ReconcilerAction { requeue_after: None })
        }
    } else {
        if obj.metadata().deletion_timestamp.is_none() {
            let patch = json_patch::Patch(if obj.metadata().finalizers.is_none() {
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
                obj.metadata().name.as_deref().unwrap(),
                &PatchParams::default(),
                &Patch::Json(patch),
            )
            .await
            .context(AddFinalizer)?;
        }
        Ok(ReconcilerAction { requeue_after: None })
    }
}
