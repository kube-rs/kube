pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use k8s_openapi::Metadata;

/// An accessor trait for Metadata
///
/// This for a subset of kubernetes type that do not end in List
/// These types, using ObjectMeta, SHOULD all have required properties:
/// - .metadata
/// - .metadata.name
/// And these optional properties:
/// - .metadata.namespace
/// - .metadata.resource_version
///
/// This avoids a bunch of the unnecessary unwrap mechanics for apps
pub trait Meta: Metadata {
    fn meta(&self) -> &ObjectMeta;
    fn name(&self) -> String;
    fn namespace(&self) -> Option<String>;
    fn resource_ver(&self) -> Option<String>;
}

/// Implement accessor trait for any ObjectMeta-using kubernetes Resource
impl<K> Meta for K
where
    K: Metadata<Ty = ObjectMeta>,
{
    fn meta(&self) -> &ObjectMeta {
        self.metadata().expect("kind has metadata")
    }

    fn name(&self) -> String {
        self.meta().name.clone().expect("kind has metadata.name")
    }

    fn resource_ver(&self) -> Option<String> {
        self.meta().resource_version.clone()
    }

    fn namespace(&self) -> Option<String> {
        self.meta().namespace.clone()
    }
}
