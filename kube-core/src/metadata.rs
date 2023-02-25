//! Metadata structs used in traits, lists, and dynamic objects.
use std::borrow::Cow;

pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};

use crate::Resource;

/// Type information that is flattened into every kubernetes object
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

/// A generic representation of any object with `ObjectMeta`.
///
/// It allows clients to get access to a particular `ObjectMeta`
/// schema without knowing the details of the version.
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartialObjectMeta<K> {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Standard object's metadata
    #[serde(default)]
    pub metadata: ObjectMeta,
    /// Type information for static dispatch
    #[serde(skip, default)]
    pub _phantom: std::marker::PhantomData<K>
}

impl<K> From<PartialObjectMeta<K>> for ObjectMeta {
    fn from(obj: PartialObjectMeta<K>) -> Self {
        ObjectMeta { ..obj.metadata }
    }
}

impl<K: Resource> Resource for PartialObjectMeta<K> {
    type DynamicType = K::DynamicType;
    type Scope = K::Scope;

    fn kind(dt: &Self::DynamicType) -> Cow<'_, str> {
        K::kind(dt)
    }

    fn group(dt: &Self::DynamicType) -> Cow<'_, str> {
        K::group(dt)
    }

    fn version(dt: &Self::DynamicType) -> Cow<'_, str> {
        K::version(dt)
    }

    fn plural(dt: &Self::DynamicType) -> Cow<'_, str> {
        K::plural(dt)
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}
