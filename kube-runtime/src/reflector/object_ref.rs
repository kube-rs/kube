use derivative::Derivative;
use k8s_openapi::{api::core::v1::ObjectReference, apimachinery::pkg::apis::meta::v1::OwnerReference};
use kube_client::{api::Resource, core::ObjectMeta, ResourceExt};
use std::fmt::{Debug, Display};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]

/// A typed and namedspaced (if relevant) reference to a Kubernetes object
///
/// ```
/// use kube_runtime::reflector::ObjectRef;
/// use k8s_openapi::api::core::v1::{ConfigMap, Secret};
/// assert_ne!(
///     ObjectRef::<ConfigMap>::new("a").erase(),
///     ObjectRef::<Secret>::new("a").erase(),
/// );
/// ```
#[non_exhaustive]
pub struct ObjectRef {
    /// The name of the object
    pub name: String,
    /// The namespace of the object
    ///
    /// May only be `None` if the kind is cluster-scoped (not located in a namespace).
    /// Note that it *is* acceptable for an `ObjectRef` to a cluster-scoped resource to
    /// have a namespace. These are, however, not considered equal:
    ///
    /// ```
    /// # use kube_runtime::reflector::ObjectRef;
    /// # use k8s_openapi::api::core::v1::ConfigMap;
    /// assert_ne!(ObjectRef::<ConfigMap>::new("foo"), ObjectRef::new("foo").within("bar"));
    /// ```
    pub namespace: Option<String>,
    /// Extra information about the object being referred to
    ///
    /// This is *not* considered when comparing objects, but may be used when converting to and from other representations,
    /// such as [`OwnerReference`] or [`ObjectReference`].
    #[derivative(Hash = "ignore", PartialEq = "ignore")]
    pub extra: Extra,
}

/// Non-vital information about an object being referred to
///
/// See [`ObjectRef::extra`].
#[derive(Default, Debug, Clone)]
#[non_exhaustive]
pub struct Extra {
    /// The version of the resource at the time of reference
    pub resource_version: Option<String>,
    /// The uid of the object
    pub uid: Option<String>,
}

impl ObjectRef {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            extra: Extra::default(),
        }
    }

    #[must_use]
    pub fn within(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    /// Creates `ObjectRef` from the resource
    #[must_use]
    pub fn from_obj<K: Resource>(obj: &K) -> Self {
        let meta = obj.meta();
        Self {
            name: obj.name_unchecked(),
            namespace: meta.namespace.clone(),
            extra: Extra::from_obj_meta(meta),
        }
    }

    /// Create an `ObjectRef` from an `OwnerReference`
    #[must_use]
    pub fn from_owner_ref(namespace: Option<&str>, owner: &OwnerReference) -> Self {
        Self {
            name: owner.name.clone(),
            namespace: namespace.map(String::from),
            extra: Extra {
                resource_version: None,
                uid: Some(owner.uid.clone()),
            },
        }
    }
}

// NB: impossible to upcast from ObjectRef to ObjectReference now without DynamicType
// impl<K: Resource> From<ObjectRef> for ObjectReference {
//     fn from(val: ObjectRef) -> Self {
//         let ObjectRef {
//             name,
//             namespace,
//             extra: Extra {
//                 resource_version,
//                 uid,
//             },
//         } = val;
//         ObjectReference {
//             api_version: Some(K::api_version(&dt).into_owned()),
//             kind: Some(K::kind(&dt).into_owned()),
//             field_path: None,
//             name: Some(name),
//             namespace,
//             resource_version,
//             uid,
//         }
//     }
// }

impl Display for ObjectRef {
    // TODO: fmt should be combined with reflector fmt so it can include the GVK
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(namespace) = &self.namespace {
            write!(f, ".{namespace}")?;
        }
        Ok(())
    }
}

impl Extra {
    fn from_obj_meta(obj_meta: &ObjectMeta) -> Self {
        Self {
            resource_version: obj_meta.resource_version.clone(),
            uid: obj_meta.uid.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use super::{Extra, ObjectRef};
    use k8s_openapi::api::{
        apps::v1::Deployment,
        core::v1::{Node, Pod},
    };

    #[test]
    fn display_should_follow_expected_format() {
        assert_eq!(
            format!("{}", ObjectRef::<Pod>::new("my-pod").within("my-namespace")),
            "Pod.v1./my-pod.my-namespace"
        );
        assert_eq!(
            format!(
                "{}",
                ObjectRef::<Deployment>::new("my-deploy").within("my-namespace")
            ),
            "Deployment.v1.apps/my-deploy.my-namespace"
        );
        assert_eq!(
            format!("{}", ObjectRef::<Node>::new("my-node")),
            "Node.v1./my-node"
        );
    }

    #[test]
    fn display_should_be_transparent_to_representation() {
        let pod_ref = ObjectRef::<Pod>::new("my-pod").within("my-namespace");
        assert_eq!(format!("{pod_ref}"), format!("{}", pod_ref.erase()));
        let deploy_ref = ObjectRef::<Deployment>::new("my-deploy").within("my-namespace");
        assert_eq!(format!("{deploy_ref}"), format!("{}", deploy_ref.erase()));
        let node_ref = ObjectRef::<Node>::new("my-node");
        assert_eq!(format!("{node_ref}"), format!("{}", node_ref.erase()));
    }

    #[test]
    fn comparison_should_ignore_extra() {
        let minimal = ObjectRef::<Pod>::new("my-pod").within("my-namespace");
        let with_extra = ObjectRef {
            extra: Extra {
                resource_version: Some("123".to_string()),
                uid: Some("638ffacd-f666-4402-ba10-7848c66ef576".to_string()),
            },
            ..minimal.clone()
        };

        // Eq and PartialEq should be unaffected by the contents of `extra`
        assert_eq!(minimal, with_extra);

        // Hash should be unaffected by the contents of `extra`
        let hash_value = |value: &ObjectRef<Pod>| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash_value(&minimal), hash_value(&with_extra));
    }
}
