//! Metadata structs used in traits, lists, and dynamic objects.
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};

use crate::ObjectList;

/// Type information that is flattened into every kubernetes object
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

/// Generic representation of any object with ObjectMeta. Allows clients to get
/// access to a particular ObjectMeta schema without knowing the details of the
/// version
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartialObjectMeta {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Object metadata
    #[serde(default)]
    pub metadata: ObjectMeta,
}

/// Represents a list of objects and contains only the ObjectMeta
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct PartialObjectMetaList {
    /// The type fields
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,

    /// List metadata
    pub metadata: ListMeta,

    /// ObjectMeta for contained objects
    pub items: Vec<PartialObjectMeta>,
}

impl From<PartialObjectMeta> for ObjectMeta {
    fn from(obj: PartialObjectMeta) -> Self {
        ObjectMeta { ..obj.metadata }
    }
}

impl From<PartialObjectMetaList> for ObjectList<ObjectMeta> {
    fn from(list: PartialObjectMetaList) -> Self {
        ObjectList {
            metadata: list.metadata,
            items: list
                .items
                .into_iter()
                .map(|item| item.into())
                .collect::<Vec<ObjectMeta>>(),
        }
    }
}
