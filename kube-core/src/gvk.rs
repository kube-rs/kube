//! Type information structs for dynamic resources.
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to parse group version: {0}")]
/// Failed to parse group version
pub struct ParseGroupVersionError(pub String);

/// Possible errors when inferring GVKs
#[derive(Debug, Error)]
pub enum GvkYamlError {
    /// Failing to parse the apiVersion
    #[error("ParseGroupVersionError: {0}")]
    ParseGroupVersion(#[source] ParseGroupVersionError),

    /// Failing to find or parse the expected apiVersion + kind
    #[error("InvalidDocument: {0}")]
    InvalidDocument(String),
}

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

    /// Extract a GroupVersionKind from a yaml document
    ///
    /// ```rust
    /// # use kube::core::GroupVersionKind;
    /// # fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// let doc = serde_yaml::from_str(r#"---
    /// apiVersion: kube.rs/v1
    /// kind: Example
    /// metadata:
    ///   name: doc1"#)?;
    ///
    /// let gvk = GroupVersionKind::from_yaml(&doc)?;
    /// assert_eq!(gvk.group, "kube.rs");
    /// assert_eq!(gvk.version, "v1");
    /// assert_eq!(gvk.kind, "Example");
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_yaml(doc: &serde_yaml::Value) -> Result<Self, GvkYamlError> {
        if let (Some(avv), Some(kv)) = (doc.get("apiVersion"), doc.get("kind")) {
            return if let (Some(apiver), Some(kind)) = (avv.as_str(), kv.as_str()) {
                let gvk = GroupVersion::from_str(apiver)
                    .map_err(GvkYamlError::ParseGroupVersion)?
                    .with_kind(kind);
                Ok(gvk)
            } else {
                let err = format!("invalid apiVersion/kind: {:?}:{:?}", avv, kv);
                Err(GvkYamlError::InvalidDocument(err))
            };
        }
        Err(GvkYamlError::InvalidDocument("missing apiVersion or kind".into()))
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

    /// Upgrade a GroupVersion to a GroupVersionKind
    pub fn with_kind(self, kind: &str) -> GroupVersionKind {
        GroupVersionKind {
            group: self.group,
            version: self.version,
            kind: kind.into(),
        }
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

mod test {
    #[test]
    fn gvk_yaml() {
        use crate::GroupVersionKind;
        use serde::Deserialize;
        let input = r#"
---
apiVersion: kube.rs/v1
kind: Example
metadata:
  name: doc1
---
apiVersion: kube.rs/v1
kind: Other
metadata:
  name: doc2"#;

        let mut gvks = vec![];
        for de in serde_yaml::Deserializer::from_str(input) {
            let doc = serde_yaml::Value::deserialize(de).unwrap();
            gvks.push(GroupVersionKind::from_yaml(&doc).unwrap());
        }
        assert_eq!(gvks[0].kind, "Example");
        assert_eq!(gvks[1].group, "kube.rs");
    }
}
