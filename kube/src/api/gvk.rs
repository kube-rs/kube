use crate::{Error, Result};
use serde::{Deserialize, Serialize};


/// Contains enough information to identify API Resource.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupVersionKind {
    /// API group
    pub group: String,
    /// Version
    pub version: String,
    /// Kind
    pub kind: String,
}

impl GroupVersionKind {
    /// Set the api group, version, and kind for a resource
    pub fn gvk(group_: &str, version_: &str, kind_: &str) -> Result<Self> {
        let version = version_.to_string();
        let group = group_.to_string();
        let kind = kind_.to_string();
        if version.is_empty() {
            return Err(Error::DynamicType(format!(
                "GroupVersionKind '{}' must have a version",
                kind
            )));
        }
        if kind.is_empty() {
            return Err(Error::DynamicType(format!(
                "GroupVersionKind '{}' must have a kind",
                kind
            )));
        }
        Ok(Self { group, version, kind })
    }
}


/// Represents a type-erased object resource.
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupVersionResource {
    /// API group
    pub group: String,
    /// Version
    pub version: String,
    /// Resource
    pub resource: String,
    /// Concatenation of group and version
    #[serde(default)]
    api_version: String,
}

impl GroupVersionResource {
    /// Set the api group, version, and the plural resource name.
    pub fn gvr(group_: &str, version_: &str, resource_: &str) -> Result<Self> {
        let version = version_.to_string();
        let group = group_.to_string();
        let resource = resource_.to_string();
        let api_version = if group.is_empty() {
            version.to_string()
        } else {
            format!("{}/{}", group, version)
        };
        if version.is_empty() {
            return Err(Error::DynamicType(format!(
                "GroupVersionResource '{}' must have a version",
                resource
            )));
        }
        if resource.is_empty() {
            return Err(Error::DynamicType(format!(
                "GroupVersionResource '{}' must have a resource",
                resource
            )));
        }
        Ok(Self {
            group,
            version,
            resource,
            api_version,
        })
    }
}
