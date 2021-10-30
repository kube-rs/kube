//! Type information structs for dynamic resources.
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to parse group version: {0}")]
/// Failed to parse group version.
pub struct ParseGroupVersionError(pub String);

/// Core information about an API Resource.
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
    /// Construct from explicit group, version, and kind
    pub fn gvk(group_: &str, version_: &str, kind_: &str) -> Self {
        let version = version_.to_string();
        let group = group_.to_string();
        let kind = kind_.to_string();

        Self { group, version, kind }
    }
}

/// Core information about a family of API Resources
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroupVersion {
    /// API group
    pub group: String,
    /// Version
    pub version: String,
}

impl GroupVersion {
    /// Construct from explicit group and version
    pub fn gv(group_: &str, version_: &str) -> Self {
        let version = version_.to_string();
        let group = group_.to_string();
        Self { group, version }
    }
}

impl FromStr for GroupVersion {
    type Err = ParseGroupVersionError;

    fn from_str(gv: &str) -> Result<Self, Self::Err> {
        let gvsplit = gv.splitn(2, '/').collect::<Vec<_>>();
        let (group, version) = match *gvsplit.as_slice() {
            [g, v] => (g.to_string(), v.to_string()), // standard case
            [v] => ("".to_string(), v.to_string()),   // core v1 case
            _ => return Err(ParseGroupVersionError(gv.into())),
        };
        Ok(Self { group, version })
    }
}

impl GroupVersion {
    /// Generate the apiVersion string used in a kind's yaml
    pub fn api_version(&self) -> String {
        if self.group.is_empty() {
            self.version.clone()
        } else {
            format!("{}/{}", self.group, self.version)
        }
    }
}
impl GroupVersionKind {
    /// Generate the apiVersion string used in a kind's yaml
    pub fn api_version(&self) -> String {
        if self.group.is_empty() {
            self.version.clone()
        } else {
            format!("{}/{}", self.group, self.version)
        }
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
    pub fn gvr(group_: &str, version_: &str, resource_: &str) -> Self {
        let version = version_.to_string();
        let group = group_.to_string();
        let resource = resource_.to_string();
        let api_version = if group.is_empty() {
            version.to_string()
        } else {
            format!("{}/{}", group, version)
        };

        Self {
            group,
            version,
            resource,
            api_version,
        }
    }
}
