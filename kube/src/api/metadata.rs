pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use k8s_openapi::Metadata;
use serde::{Deserialize, Serialize};

/// An accessor trait for Metadata
///
/// This for a subset of Kubernetes type that do not end in List
/// These types, using ObjectMeta, SHOULD all have required properties:
/// - .metadata
/// - .metadata.name
/// And these optional properties:
/// - .metadata.namespace
/// - .metadata.resource_version
pub trait Meta: Metadata {
    /// Metadata that all persisted resources must have
    fn meta(&self) -> &ObjectMeta;
    /// The name of the resource
    fn name(&self) -> &str;
    /// The namespace the resource is in
    fn namespace(&self) -> Option<&str>;
    /// Tthe resource version
    fn resource_ver(&self) -> Option<&str>;
}

/// Implement accessor trait for any ObjectMeta-using Kubernetes Resource
impl<K> Meta for K
where
    K: Metadata<Ty = ObjectMeta>,
{
    fn meta(&self) -> &ObjectMeta {
        self.metadata()
    }

    fn name(&self) -> &str {
        self.meta().name.as_ref().expect("kind has metadata.name")
    }

    fn resource_ver(&self) -> Option<&str> {
        self.meta().resource_version.as_deref()
    }

    fn namespace(&self) -> Option<&str> {
        self.meta().namespace.as_deref()
    }
}

/// A convenience struct for ad-hoc serialization
///
/// Mostly useful for `Object`
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}
