//! Generic helpers for managing owner references between resources.
use crate::resource::{Resource, ResourceExt};

/// Error returned by [`set_controller_reference`] when `controlled` is already
/// controlled by a different owner.
#[derive(Debug, thiserror::Error)]
#[error(
    "object already has a different controller (uid {existing_uid:?}), cannot set controller (uid {new_uid:?})"
)]
pub struct AlreadyOwnedError {
    /// The `uid` of the owner reference already marked as controller on the object
    pub existing_uid: String,
    /// The `uid` of the new owner that was requested to become the controller
    pub new_uid: String,
}

/// Returns whether `controlled` has an owner reference with the given `uid`
///
/// ```
/// use k8s_openapi::api::core::v1::{ConfigMap, Pod};
/// use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
/// use kube_core::{has_owner_reference, set_owner_reference};
///
/// let pod = Pod {
///     metadata: ObjectMeta {
///         name: Some("my-pod".to_string()),
///         uid: Some("pod-uid".to_string()),
///         ..Default::default()
///     },
///     ..Default::default()
/// };
/// let mut cm = ConfigMap::default();
/// assert!(!has_owner_reference(&cm, "pod-uid"));
/// set_owner_reference(&pod, &(), &mut cm);
/// assert!(has_owner_reference(&cm, "pod-uid"));
/// ```
pub fn has_owner_reference<K: Resource>(controlled: &K, uid: &str) -> bool {
    controlled.owner_references().iter().any(|o| o.uid == uid)
}

/// Sets `owner` as a (non-controlling) owner reference on `controlled`
///
/// If `controlled` already has an owner reference with the same `uid`, it is replaced in place;
/// otherwise the new owner reference is appended.
///
/// Returns `None` if `owner` does not have a `.metadata.name`/`.metadata.uid` set yet (see
/// [`Resource::owner_ref`]), in which case `controlled` is left unchanged.
pub fn set_owner_reference<Owner: Resource, K: Resource>(
    owner: &Owner,
    dt: &Owner::DynamicType,
    controlled: &mut K,
) -> Option<()> {
    let new_ref = owner.owner_ref(dt)?;
    upsert_owner_reference(controlled, new_ref);
    Some(())
}

/// Sets `owner` as the controller reference on `controlled`
///
/// Errors with [`AlreadyOwnedError`] if `controlled` already has a controller reference pointing
/// at a *different* owner. Idempotent (a no-op returning `Ok(())`) if the existing controller
/// reference already points at `owner`.
///
/// Returns `Ok(())` without changes if `owner` does not have a `.metadata.name`/`.metadata.uid` set yet.
pub fn set_controller_reference<Owner: Resource, K: Resource>(
    owner: &Owner,
    dt: &Owner::DynamicType,
    controlled: &mut K,
) -> Result<(), AlreadyOwnedError> {
    let Some(new_ref) = owner.controller_owner_ref(dt) else {
        return Ok(());
    };
    if let Some(existing) = controlled
        .owner_references()
        .iter()
        .find(|o| o.controller == Some(true))
        && existing.uid != new_ref.uid
    {
        return Err(AlreadyOwnedError {
            existing_uid: existing.uid.clone(),
            new_uid: new_ref.uid,
        });
    }
    upsert_owner_reference(controlled, new_ref);
    Ok(())
}

fn upsert_owner_reference<K: Resource>(
    controlled: &mut K,
    new_ref: k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference,
) {
    let refs = controlled.owner_references_mut();
    if let Some(existing) = refs.iter_mut().find(|o| o.uid == new_ref.uid) {
        *existing = new_ref;
    } else {
        refs.push(new_ref);
    }
}

#[cfg(test)]
mod tests {
    use super::{has_owner_reference, set_controller_reference, set_owner_reference};
    use crate::resource::ResourceExt;
    use k8s_openapi::{
        api::core::v1::{ConfigMap, Pod},
        apimachinery::pkg::apis::meta::v1::ObjectMeta,
    };

    fn pod(name: &str, uid: &str) -> Pod {
        Pod {
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                uid: Some(uid.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn set_owner_reference_appends_and_replaces() {
        let owner = pod("owner", "owner-uid");
        let mut cm = ConfigMap::default();

        set_owner_reference(&owner, &(), &mut cm).unwrap();
        assert!(has_owner_reference(&cm, "owner-uid"));
        assert_eq!(cm.owner_references().len(), 1);

        // Setting again with the same uid replaces in place rather than duplicating
        set_owner_reference(&owner, &(), &mut cm).unwrap();
        assert_eq!(cm.owner_references().len(), 1);
    }

    #[test]
    fn set_controller_reference_is_idempotent() {
        let owner = pod("owner", "owner-uid");
        let mut cm = ConfigMap::default();

        set_controller_reference(&owner, &(), &mut cm).unwrap();
        assert!(cm.owner_references()[0].controller == Some(true));

        // Re-setting the same controller is a no-op success
        set_controller_reference(&owner, &(), &mut cm).unwrap();
        assert_eq!(cm.owner_references().len(), 1);
    }

    #[test]
    fn set_controller_reference_rejects_conflicting_owner() {
        let owner = pod("owner", "owner-uid");
        let other = pod("other", "other-uid");
        let mut cm = ConfigMap::default();

        set_controller_reference(&owner, &(), &mut cm).unwrap();
        let err = set_controller_reference(&other, &(), &mut cm).unwrap_err();
        assert_eq!(err.existing_uid, "owner-uid");
        assert_eq!(err.new_uid, "other-uid");
    }

    #[test]
    fn set_controller_reference_is_noop_for_unnamed_owner() {
        let owner = Pod::default(); // no name/uid set yet
        let mut cm = ConfigMap::default();

        set_controller_reference(&owner, &(), &mut cm).unwrap();
        assert!(cm.owner_references().is_empty());
    }

    #[test]
    fn set_owner_reference_is_noop_for_unnamed_owner() {
        let owner = Pod::default(); // no name/uid set yet
        let mut cm = ConfigMap::default();

        assert!(set_owner_reference(&owner, &(), &mut cm).is_none());
        assert!(cm.owner_references().is_empty());
    }
}
