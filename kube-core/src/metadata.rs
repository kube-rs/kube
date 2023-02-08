//! Metadata structs used in traits, lists, and dynamic objects.
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};

/// Type information that is flattened into every kubernetes object
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

/// PartialObjectMetadata is a generic representation of any object with
/// ObjectMeta. It allows clients to get access to a particular ObjectMeta
/// schema without knowing the details of the version.
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartialObjectMeta {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Standard object's metadata
    #[serde(default)]
    pub metadata: ObjectMeta,
}

impl From<PartialObjectMeta> for ObjectMeta {
    fn from(obj: PartialObjectMeta) -> Self {
        ObjectMeta { ..obj.metadata }
    }
}
