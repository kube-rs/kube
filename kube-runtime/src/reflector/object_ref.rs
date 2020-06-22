use derivative::Derivative;
use k8s_openapi::{apimachinery::pkg::apis::meta::v1::OwnerReference, Resource};
use kube::api::Meta;
use std::{fmt::Debug, hash::Hash};

#[derive(Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash, Clone)]
pub struct ObjectRef<K: RuntimeResource> {
    kind: K::State,
    pub name: String,
    pub namespace: Option<String>,
}

impl<K: Meta> ObjectRef<K> {
    #[must_use]
    pub fn new_namespaced(name: String, namespace: String) -> Self {
        Self {
            kind: (),
            name,
            namespace: Some(namespace),
        }
    }

    #[must_use]
    pub fn new_clusterscoped(name: String) -> Self {
        Self {
            kind: (),
            name,
            namespace: None,
        }
    }

    #[must_use]
    pub fn from_obj(obj: &K) -> Self {
        Self {
            kind: (),
            name: obj.name(),
            namespace: obj.namespace(),
        }
    }

    #[must_use]
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

pub trait RuntimeResource {
    type State: Debug + PartialEq + Eq + Hash + Clone;
    fn group(state: &Self::State) -> &str;
    fn version(state: &Self::State) -> &str;
    fn kind(state: &Self::State) -> &str;
}

impl<K: Resource> RuntimeResource for K {
    // All required state is provided at build time
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
