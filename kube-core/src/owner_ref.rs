//! Generic helpers for managing owner references between resources.
use crate::resource::{Resource, ResourceExt};

/// Error returned by [`OwnerRefExt::set_controller_reference`] when the object is already
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

/// Extension trait for managing owner references on any [`Resource`]
///
/// Like [`FinalizerExt`](crate::FinalizerExt), it is kept separate from [`ResourceExt`] and is
/// only in scope when imported explicitly.
///
/// ```
/// use k8s_openapi::api::core::v1::{ConfigMap, Pod};
/// use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
/// use kube_core::OwnerRefExt;
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
/// assert!(!cm.has_owner_reference("pod-uid"));
/// cm.set_owner_reference(&pod, &());
/// assert!(cm.has_owner_reference("pod-uid"));
/// ```
pub trait OwnerRefExt: Resource {
    /// Returns whether the object has an owner reference with the given `uid`
    fn has_owner_reference(&self, uid: &str) -> bool;

    /// Sets `owner` as a (non-controlling) owner reference on the object
    ///
    /// If the object already has an owner reference with the same `uid`, it is replaced in place;
    /// otherwise the new owner reference is appended.
    ///
    /// Returns `None` if `owner` does not have a `.metadata.name`/`.metadata.uid` set yet (see
    /// [`Resource::owner_ref`]), in which case the object is left unchanged.
    fn set_owner_reference<Owner: Resource>(&mut self, owner: &Owner, dt: &Owner::DynamicType) -> Option<()>;

    /// Sets `owner` as the controller reference on the object
    ///
    /// Errors with [`AlreadyOwnedError`] if the object already has a controller reference pointing
    /// at a *different* owner. Idempotent (a no-op returning `Ok(())`) if the existing controller
    /// reference already points at `owner`.
    ///
    /// Returns `Ok(())` without changes if `owner` does not have a `.metadata.name`/`.metadata.uid` set yet.
    fn set_controller_reference<Owner: Resource>(
        &mut self,
        owner: &Owner,
        dt: &Owner::DynamicType,
    ) -> Result<(), AlreadyOwnedError>;
}

impl<K: Resource> OwnerRefExt for K {
    fn has_owner_reference(&self, uid: &str) -> bool {
        self.owner_references().iter().any(|o| o.uid == uid)
    }

    fn set_owner_reference<Owner: Resource>(&mut self, owner: &Owner, dt: &Owner::DynamicType) -> Option<()> {
        let new_ref = owner.owner_ref(dt)?;
        upsert_owner_reference(self, new_ref);
        Some(())
    }

    fn set_controller_reference<Owner: Resource>(
        &mut self,
        owner: &Owner,
        dt: &Owner::DynamicType,
    ) -> Result<(), AlreadyOwnedError> {
        let Some(new_ref) = owner.controller_owner_ref(dt) else {
            return Ok(());
        };
        if let Some(existing) = self
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
        upsert_owner_reference(self, new_ref);
        Ok(())
    }
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
    use super::OwnerRefExt;
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

        cm.set_owner_reference(&owner, &()).unwrap();
        assert!(cm.has_owner_reference("owner-uid"));
        assert_eq!(cm.owner_references().len(), 1);

        // Setting again with the same uid replaces in place rather than duplicating
        cm.set_owner_reference(&owner, &()).unwrap();
        assert_eq!(cm.owner_references().len(), 1);
    }

    #[test]
    fn set_controller_reference_is_idempotent() {
        let owner = pod("owner", "owner-uid");
        let mut cm = ConfigMap::default();

        cm.set_controller_reference(&owner, &()).unwrap();
        assert!(cm.owner_references()[0].controller == Some(true));

        // Re-setting the same controller is a no-op success
        cm.set_controller_reference(&owner, &()).unwrap();
        assert_eq!(cm.owner_references().len(), 1);
    }

    #[test]
    fn set_controller_reference_rejects_conflicting_owner() {
        let owner = pod("owner", "owner-uid");
        let other = pod("other", "other-uid");
        let mut cm = ConfigMap::default();

        cm.set_controller_reference(&owner, &()).unwrap();
        let err = cm.set_controller_reference(&other, &()).unwrap_err();
        assert_eq!(err.existing_uid, "owner-uid");
        assert_eq!(err.new_uid, "other-uid");
    }

    #[test]
    fn set_controller_reference_is_noop_for_unnamed_owner() {
        let owner = Pod::default(); // no name/uid set yet
        let mut cm = ConfigMap::default();

        cm.set_controller_reference(&owner, &()).unwrap();
        assert!(cm.owner_references().is_empty());
    }

    #[test]
    fn set_owner_reference_is_noop_for_unnamed_owner() {
        let owner = Pod::default(); // no name/uid set yet
        let mut cm = ConfigMap::default();

        assert!(cm.set_owner_reference(&owner, &()).is_none());
        assert!(cm.owner_references().is_empty());
    }
}
