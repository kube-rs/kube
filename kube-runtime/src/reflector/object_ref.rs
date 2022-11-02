use derivative::Derivative;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use kube_client::core::{ObjectMeta, TypeInfo, TypeMeta};
use std::fmt::{Debug, Display};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]

/// A named, typed and namedspaced reference to a Kubernetes object
///
/// ```
/// use kube_client::core::{Resource};
/// use k8s_openapi::api::{apps::v1::Deployment, core::v1::Pod};
/// use kube_runtime::reflector::ObjectRef;
/// assert_ne!(
///     ObjectRef::new("a").within("ns").with_types(&<Deployment as Resource>::typemeta().unwrap()),
///     ObjectRef::new("a").within("ns").with_types(&<Pod as Resource>::typemeta().unwrap()),
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
    /// assert_ne!(ObjectRef::new("foo"), ObjectRef::new("foo").within("bar"));
    /// ```
    pub namespace: Option<String>,
    /// The TypeMeta of the object
    pub types: Option<TypeMeta>,
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
    /// Create a blank `ObjectRef` with a name
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            namespace: None,
            types: None,
            extra: Extra::default(),
        }
    }

    /// Creates `ObjectRef` from the resource
    #[must_use]
    pub fn from_obj<K: TypeInfo>(obj: &K) -> Self {
        let meta = obj.meta();
        Self {
            name: meta.name.clone().unwrap(),
            namespace: meta.namespace.clone(),
            types: obj.types(),
            extra: Extra::from_objectmeta(meta),
        }
    }

    /// Creates a partial `ObjectRef` from an `OwnerReference`
    #[must_use]
    pub fn from_owner(owner: &OwnerReference) -> Self {
        Self {
            name: owner.name.clone(),
            namespace: None,
            types: None,
            extra: Extra {
                resource_version: None,
                uid: Some(owner.uid.clone()),
            },
        }
    }

    #[must_use]
    pub fn within(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    #[must_use]
    pub fn with_namespace(mut self, namespace: Option<&str>) -> Self {
        self.namespace = namespace.map(String::from);
        self
    }

    #[must_use]
    pub fn with_owner(mut self, owner: &OwnerReference) -> Self {
        self.extra.uid = Some(owner.uid.clone());
        self
    }

    #[must_use]
    pub fn with_types(mut self, types: &TypeMeta) -> Self {
        self.types = Some(types.clone());
        self
    }
}

// NB: impossible to upcast from ObjectReference to ObjectRef now without DynamicType
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tm) = &self.types {
            write!(f, "{}.{}/{}", tm.kind, tm.api_version, self.name)?;
        } else {
            write!(f, "{}", self.name)?;
        }
        if let Some(namespace) = &self.namespace {
            write!(f, ".{namespace}")?;
        }
        Ok(())
    }
}

impl Extra {
    fn from_objectmeta(meta: &ObjectMeta) -> Self {
        Self {
            resource_version: meta.resource_version.clone(),
            uid: meta.uid.clone(),
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
    use k8s_openapi::api::{apps::v1::Deployment, core::v1::Pod};
    use kube_client::{core::TypeMeta, Resource};


    #[test]
    fn display_should_follow_expected_format() {
        let pod_type = <Pod as Resource>::typemeta().unwrap();
        assert_eq!(
            format!(
                "{}",
                ObjectRef::new("my-pod")
                    .within("my-namespace")
                    .with_types(&pod_type)
            ),
            "Pod.v1/my-pod.my-namespace"
        );
        let deploy_type = <Deployment as Resource>::typemeta().unwrap();
        assert_eq!(
            format!(
                "{}",
                ObjectRef::new("my-deploy")
                    .within("my-namespace")
                    .with_types(&deploy_type)
            ),
            "Deployment.apps/v1/my-deploy.my-namespace"
        );
    }

    #[test]
    fn comparison_should_ignore_extra() {
        let minimal = ObjectRef::new("my-pod").within("my-namespace");
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
        let hash_value = |value: &ObjectRef| {
            let mut hasher = DefaultHasher::new();
            value.hash(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash_value(&minimal), hash_value(&with_extra));
    }
}
