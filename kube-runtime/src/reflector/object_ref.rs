use derivative::Derivative;
use k8s_openapi::{api::core::v1::ObjectReference, apimachinery::pkg::apis::meta::v1::OwnerReference};
use kube_client::core::{Inspect, ObjectMeta, Resource, TypeMeta};
use std::fmt::{Debug, Display};
use thiserror::Error;

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]

/// A dynamically typed reference to an object along with its namespace
///
/// Intended to be constructed from one of three sources:
/// 1. an object returned by the apiserver through the `Inspect` trait
/// 2. an `OwnerReference` found on an object returned by the apiserver
/// 3. a type implementing `Inspect` but with only a `name` pointing to the type
///
/// ```
/// use kube_client::core::Resource;
/// use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
/// # use k8s_openapi::api::core::v1::Pod;
/// # use kube_runtime::reflector::ObjectRef;
/// let mut pod = Pod::default();
/// pod.meta_mut().name = Some("foo".into());
/// let oref = OwnerReference {
///    api_version: "v1".into(),
///    kind: "Pod".into(),
///    name: "foo".into(),
///    ..OwnerReference::default()
/// };
/// assert_eq!(
///     ObjectRef::from_obj(&pod).within("ns"),
///     ObjectRef::from_owner(&oref).within("ns"),
/// );
/// assert_eq!(
///     ObjectRef::from_obj(&pod).within("ns"),
///     ObjectRef::from_resource::<Pod>("foo").within("ns")
/// );
/// ```
#[non_exhaustive]
pub struct ObjectRef {
    /// The name of the object
    pub name: String,
    /// The namespace of the object
    ///
    /// May only be `None` if the kind is cluster-scoped (not located in a namespace).
    ///
    /// When constructing an `ObjectRef` be sure to either:
    ///
    /// 1. supply a **known** namespace using `ObjectRef::within`
    /// 2. supply an **optional** namespace using `ObjectRef::with_namespace` (when being generic over kinds)
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
    /// Creates an `ObjectRef` from an object implementing `Inspect`
    #[must_use]
    pub fn from_obj<K: Inspect>(obj: &K) -> Self {
        let meta = obj.meta();
        Self {
            name: meta.name.clone().unwrap_or_default(),
            namespace: meta.namespace.clone(),
            types: obj.types(),
            extra: Extra::from_objectmeta(meta),
        }
    }

    /// Creates an `ObjectRef` from an `OwnerReference`
    #[must_use]
    pub fn from_owner(owner: &OwnerReference) -> Self {
        Self {
            name: owner.name.clone(),
            namespace: None,
            types: Some(TypeMeta {
                api_version: owner.api_version.clone(),
                kind: owner.kind.clone(),
            }),
            extra: Extra {
                resource_version: None,
                uid: Some(owner.uid.clone()),
            },
        }
    }

    /// Creates an `ObjectRef` from a `Resource` along with name
    #[must_use]
    pub fn from_resource<K: Resource>(name: &str) -> Self {
        Self {
            name: name.to_string(),
            namespace: None,
            types: K::typemeta(),
            extra: Extra::default(),
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
    pub fn with_types(mut self, types: &TypeMeta) -> Self {
        self.types = Some(types.clone());
        self
    }
}

#[derive(Debug, Error)]
#[error("missing type information from ObjectRef")]
/// Source does not have `TypeMeta`
pub struct MissingTypeInfo;


impl TryFrom<ObjectRef> for ObjectReference {
    type Error = MissingTypeInfo;

    fn try_from(val: ObjectRef) -> Result<Self, Self::Error> {
        if let Some(t) = &val.types {
            let ObjectRef {
                name,
                namespace,
                extra:
                    Extra {
                        resource_version,
                        uid,
                    },
                ..
            } = val;
            Ok(ObjectReference {
                api_version: Some(t.api_version.clone()),
                kind: Some(t.kind.clone()),
                field_path: None,
                name: Some(name),
                namespace,
                resource_version,
                uid,
            })
        } else {
            Err(MissingTypeInfo)
        }
    }
}

impl Display for ObjectRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(tm) = &self.types {
            write!(f, "{}.{}/{}", tm.kind, tm.api_version, self.name)?;
        } else {
            write!(f, "unknown/{}", self.name)?;
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
    use kube_client::Resource;

    #[test]
    fn display_should_follow_expected_format() {
        let pod = ObjectRef::from_resource::<Pod>("my-pod").within("my-ns");
        assert_eq!(format!("{}", pod), "Pod.v1/my-pod.my-ns");
        let deploy = ObjectRef::from_resource::<Deployment>("my-dep").within("my-ns");
        assert_eq!(format!("{}", deploy), "Deployment.apps/v1/my-dep.my-ns");
    }

    #[test]
    fn comparison_should_ignore_extra() {
        let mut pod = Pod::default();
        pod.meta_mut().name = Some("my-pod".into());
        pod.meta_mut().namespace = Some("my-namespace".into());
        let minimal = ObjectRef::from_obj(&pod);
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
