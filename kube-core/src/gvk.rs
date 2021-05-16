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
    pub fn gvk(group_: &str, version_: &str, kind_: &str) -> Self {
        let version = version_.to_string();
        let group = group_.to_string();
        let kind = kind_.to_string();

        Self { group, version, kind }
    }
}
