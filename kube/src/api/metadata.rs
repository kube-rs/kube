pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};

pub use kube_core::resource::{Resource, ResourceExt};

/// A convenience struct for ad-hoc serialization.
///
/// Mostly useful for [`Object`](super::Object).
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}
