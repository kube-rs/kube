use derivative::Derivative;
use k8s_openapi::{apimachinery::pkg::apis::meta::v1::OwnerReference, Resource};
use kube::api::Meta;
use std::{fmt::Debug, hash::Hash};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]
/// A typed and namedspaced (if relevant) reference to a Kubernetes object
///
/// `K` may be either the object type or `ErasedResource`, in which case the
/// type is stored at runtime. Erased `ObjectRef`s pointing to different types
/// are still considered different.
///
/// ```
/// use kube_runtime::reflector::{ErasedResource, ObjectRef};
/// use k8s_openapi::api::core::v1::{ConfigMap, Secret};
/// assert_ne!(
///     ObjectRef::<ErasedResource>::from(ObjectRef::<ConfigMap>::new("a")),
///     ObjectRef::<ErasedResource>::from(ObjectRef::<Secret>::new("a")),
/// );
/// ```
pub struct ObjectRef<K: RuntimeResource> {
    kind: K::State,
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
}

impl<K: Resource> ObjectRef<K> {
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            kind: (),
            name: name.into(),
            namespace: None,
        }
    }

    #[must_use]
    pub fn within(mut self, namespace: &str) -> Self {
        self.namespace = Some(namespace.to_string());
        self
    }

    #[must_use]
    pub fn from_obj(obj: &K) -> Self
    where
        K: Meta,
    {
        Self {
            kind: (),
            name: obj.name(),
            namespace: obj.namespace(),
        }
    }

    #[must_use]
    /// Create an `ObjectRef` from an `OwnerReference`
    ///
    /// Returns `None` if the types do not match.
    pub fn from_owner_ref(namespace: Option<&str>, owner: &OwnerReference) -> Option<Self> {
        if owner.api_version == K::API_VERSION && owner.kind == K::KIND {
            Some(Self {
                kind: (),
                name: owner.name.clone(),
                namespace: namespace.map(String::from),
            })
        } else {
            None
        }
    }
}

/// A Kubernetes type that is known at runtime
pub trait RuntimeResource {
    type State: Debug + PartialEq + Eq + Hash + Clone;
    fn group(state: &Self::State) -> &str;
    fn version(state: &Self::State) -> &str;
    fn kind(state: &Self::State) -> &str;
}

/// All `Resource`s are also known at runtime
impl<K: Resource> RuntimeResource for K {
    /// All required state is provided at build time
    type State = ();

    fn group(_state: &Self::State) -> &str {
        K::GROUP
    }

    fn version(_state: &Self::State) -> &str {
        K::VERSION
    }

    fn kind(_state: &Self::State) -> &str {
        K::KIND
    }
}

/// Marker for indicating that the `ObjectRef`'s type is only known at runtime
// ! is still unstable: https://github.com/rust-lang/rust/issues/35121
#[allow(clippy::empty_enum)]
pub enum ErasedResource {}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct ErasedResourceState {
    group: &'static str,
    version: &'static str,
    kind: &'static str,
}
impl RuntimeResource for ErasedResource {
    type State = ErasedResourceState;

    fn group(state: &Self::State) -> &str {
        &state.group
    }

    fn version(state: &Self::State) -> &str {
        &state.version
    }

    fn kind(state: &Self::State) -> &str {
        &state.kind
    }
}

impl ErasedResource {
    fn erase<K: Resource>() -> ErasedResourceState {
        ErasedResourceState {
            group: K::GROUP,
            version: K::VERSION,
            kind: K::KIND,
        }
    }
}

impl<K: Resource> From<ObjectRef<K>> for ObjectRef<ErasedResource> {
    fn from(old: ObjectRef<K>) -> Self {
        ObjectRef {
            kind: ErasedResource::erase::<K>(),
            name: old.name,
            namespace: old.namespace,
        }
    }
}
