use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub use kube_core::resource::Resource;

/// Helper methods for resources.
pub trait ResourceExt: Resource {
    /// Returns the name of the resource, panicking if it is
    /// missing. Use this function if you know that name is set, for example
    /// when resource was received from the apiserver.
    /// Because of `.metadata.generateName` field, in other contexts name
    /// may be missing.
    ///
    /// For non-panicking alternative, you can directly read `name` field
    /// on the `self.meta()`.
    fn name(&self) -> String;
    /// The namespace the resource is in
    fn namespace(&self) -> Option<String>;
    /// The resource version
    fn resource_version(&self) -> Option<String>;
    /// Unique ID (if you delete resource and then create a new
    /// resource with the same name, it will have different ID)
    fn uid(&self) -> Option<String>;
    /// Returns resource labels
    fn labels(&self) -> &BTreeMap<String, String>;
    /// Provides mutable access to the labels
    fn labels_mut(&mut self) -> &mut BTreeMap<String, String>;
    /// Returns resource annotations
    fn annotations(&self) -> &BTreeMap<String, String>;
    /// Provider mutable access to the annotations
    fn annotations_mut(&mut self) -> &mut BTreeMap<String, String>;
    /// Returns resource owner references
    fn owner_references(&self) -> &[OwnerReference];
    /// Provides mutable access to the owner references
    fn owner_references_mut(&mut self) -> &mut Vec<OwnerReference>;
    /// Returns resource finalizers
    fn finalizers(&self) -> &[String];
    /// Provides mutable access to the finalizers
    fn finalizers_mut(&mut self) -> &mut Vec<String>;
}

// TODO: replace with ordinary static when BTreeMap::new() is no longer
// const-unstable.
static EMPTY_MAP: Lazy<BTreeMap<String, String>> = Lazy::new(BTreeMap::new);

impl<K: Resource> ResourceExt for K {
    fn name(&self) -> String {
        self.meta().name.clone().expect(".metadata.name missing")
    }

    fn namespace(&self) -> Option<String> {
        self.meta().namespace.clone()
    }

    fn resource_version(&self) -> Option<String> {
        self.meta().resource_version.clone()
    }

    fn uid(&self) -> Option<String> {
        self.meta().uid.clone()
    }

    fn labels(&self) -> &BTreeMap<String, String> {
        self.meta().labels.as_ref().unwrap_or_else(|| &*EMPTY_MAP)
    }

    fn labels_mut(&mut self) -> &mut BTreeMap<String, String> {
        self.meta_mut().labels.get_or_insert_with(BTreeMap::new)
    }

    fn annotations(&self) -> &BTreeMap<String, String> {
        self.meta().annotations.as_ref().unwrap_or_else(|| &*EMPTY_MAP)
    }

    fn annotations_mut(&mut self) -> &mut BTreeMap<String, String> {
        self.meta_mut().annotations.get_or_insert_with(BTreeMap::new)
    }

    fn owner_references(&self) -> &[OwnerReference] {
        self.meta().owner_references.as_deref().unwrap_or_default()
    }

    fn owner_references_mut(&mut self) -> &mut Vec<OwnerReference> {
        self.meta_mut().owner_references.get_or_insert_with(Vec::new)
    }

    fn finalizers(&self) -> &[String] {
        self.meta().finalizers.as_deref().unwrap_or_default()
    }

    fn finalizers_mut(&mut self) -> &mut Vec<String> {
        self.meta_mut().finalizers.get_or_insert_with(Vec::new)
    }
}

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
