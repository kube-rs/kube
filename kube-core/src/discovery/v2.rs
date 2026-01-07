//! Types for the Aggregated Discovery API (apidiscovery.k8s.io/v2)
//!
//! These types are not part of the Kubernetes OpenAPI spec, so they are defined here
//! rather than in k8s-openapi. They mirror the types from k8s.io/api/apidiscovery/v2.
//!
//! The Aggregated Discovery API is available since Kubernetes 1.26 (beta) and stable in 1.30+.

use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};

/// Content negotiation Accept header for Aggregated Discovery API v2
pub const ACCEPT_AGGREGATED_DISCOVERY_V2: &str = "application/json;g=apidiscovery.k8s.io;v=v2;as=APIGroupDiscoveryList,application/json;g=apidiscovery.k8s.io;v=v2beta1;as=APIGroupDiscoveryList,application/json";


/// APIGroupDiscoveryList is a resource containing a list of APIGroupDiscovery.
/// This is one of the types that can be returned from the /api and /apis endpoint
/// and contains an aggregated list of API resources (built-ins, Custom Resource Definitions, resources from aggregated servers)
/// that a cluster supports.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct APIGroupDiscoveryList {
    /// Standard list metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ListMeta>,

    /// items is the list of groups for discovery.
    /// The groups are listed in priority order.
    #[serde(default)]
    pub items: Vec<APIGroupDiscovery>,
}

/// APIGroupDiscovery holds information about which resources are being served for all version of the API Group.
/// It contains a list of APIVersionDiscovery that holds a list of APIResourceDiscovery types served for a version.
/// Versions are in descending order of preference, with the first version being the preferred entry.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct APIGroupDiscovery {
    /// Standard object's metadata.
    /// The only field populated will be name. It will be the name of the API group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ObjectMeta>,

    /// versions are the versions supported in this group.
    /// They are sorted in descending order of preference,
    /// with the preferred version being the first entry.
    #[serde(default)]
    pub versions: Vec<APIVersionDiscovery>,
}

/// APIVersionDiscovery holds a list of APIResourceDiscovery types that are served for a particular version within an API Group.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct APIVersionDiscovery {
    /// version is the name of the version within a group version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// resources is a list of APIResourceDiscovery objects for the corresponding group version.
    #[serde(default)]
    pub resources: Vec<APIResourceDiscovery>,

    /// freshness marks whether a group version's discovery document is up to date.
    /// "Current" indicates the discovery document was recently refreshed.
    /// "Stale" indicates the discovery document could not be retrieved and
    /// the returned discovery document may be significantly out of date.
    /// Clients that require the latest version of the discovery information
    /// should not use the aggregated document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub freshness: Option<String>,
}

/// APIResourceDiscovery provides information about an API resource for discovery.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct APIResourceDiscovery {
    /// resource is the plural name of the resource.
    /// This is used in the URL path and is the unique identifier for this resource across all versions in the API group.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,

    /// responseKind describes the group, version, and kind of the serialization schema for the object type this endpoint typically returns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_kind: Option<GroupVersionKind>,

    /// scope indicates the scope of a resource, either "Cluster" or "Namespaced".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,

    /// singularResource is the singular name of the resource.
    /// This allows clients to handle plural and singular opaquely.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub singular_resource: Option<String>,

    /// verbs is a list of supported API operation types (this includes but is not limited to get, list, watch, create, update, patch, delete, deletecollection, and proxy).
    #[serde(default)]
    pub verbs: Vec<String>,

    /// shortNames is a list of suggested short names of the resource.
    #[serde(default)]
    pub short_names: Vec<String>,

    /// categories is a list of the grouped resources this resource belongs to (e.g. 'all').
    /// Clients may use this to simplify acting on multiple resource types at once.
    #[serde(default)]
    pub categories: Vec<String>,

    /// subresources is a list of subresources provided by this resource.
    /// Subresources are located at /api/v1/namespaces/{namespace}/{resource}/{name}/{subresource}
    #[serde(default)]
    pub subresources: Vec<APISubresourceDiscovery>,
}

/// APISubresourceDiscovery provides information about an API subresource for discovery.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct APISubresourceDiscovery {
    /// subresource is the name of the subresource.
    /// This is used in the URL path and is the unique identifier for this resource across all versions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subresource: Option<String>,

    /// responseKind describes the group, version, and kind of the serialization schema for the object type this endpoint typically returns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_kind: Option<GroupVersionKind>,

    /// acceptedTypes describes the kinds that this endpoint accepts.
    /// Subresources may accept the parent's kind (for update, patch) or its own kind (for create).
    #[serde(default)]
    pub accepted_types: Vec<GroupVersionKind>,

    /// verbs is a list of supported API operation types (this includes but is not limited to get, list, watch, create, update, patch, delete).
    #[serde(default)]
    pub verbs: Vec<String>,
}

/// GroupVersionKind unambiguously identifies a kind.
/// This is a local copy for use in discovery types.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupVersionKind {
    /// group is the group of the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,

    /// version is the version of the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// kind is the kind of the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_api_group_discovery_list() {
        // Sample response similar to what Kubernetes returns from /apis with aggregated discovery
        let json = r#"{
            "kind": "APIGroupDiscoveryList",
            "apiVersion": "apidiscovery.k8s.io/v2",
            "metadata": {},
            "items": [
                {
                    "metadata": {
                        "name": "apps"
                    },
                    "versions": [
                        {
                            "version": "v1",
                            "freshness": "Current",
                            "resources": [
                                {
                                    "resource": "deployments",
                                    "responseKind": {
                                        "group": "apps",
                                        "version": "v1",
                                        "kind": "Deployment"
                                    },
                                    "scope": "Namespaced",
                                    "singularResource": "deployment",
                                    "verbs": ["create", "delete", "deletecollection", "get", "list", "patch", "update", "watch"],
                                    "shortNames": ["deploy"],
                                    "categories": ["all"],
                                    "subresources": [
                                        {
                                            "subresource": "status",
                                            "responseKind": {
                                                "group": "apps",
                                                "version": "v1",
                                                "kind": "Deployment"
                                            },
                                            "verbs": ["get", "patch", "update"]
                                        },
                                        {
                                            "subresource": "scale",
                                            "responseKind": {
                                                "group": "autoscaling",
                                                "version": "v1",
                                                "kind": "Scale"
                                            },
                                            "verbs": ["get", "patch", "update"]
                                        }
                                    ]
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let result: APIGroupDiscoveryList = serde_json::from_str(json).unwrap();

        assert_eq!(result.items.len(), 1);

        let apps_group = &result.items[0];
        assert_eq!(
            apps_group.metadata.as_ref().and_then(|m| m.name.as_ref()),
            Some(&"apps".to_string())
        );

        assert_eq!(apps_group.versions.len(), 1);
        let v1 = &apps_group.versions[0];
        assert_eq!(v1.version, Some("v1".to_string()));
        assert_eq!(v1.freshness, Some("Current".to_string()));

        assert_eq!(v1.resources.len(), 1);
        let deployments = &v1.resources[0];
        assert_eq!(deployments.resource, Some("deployments".to_string()));
        assert_eq!(deployments.scope, Some("Namespaced".to_string()));
        assert_eq!(deployments.singular_resource, Some("deployment".to_string()));
        assert_eq!(deployments.short_names, vec!["deploy"]);
        assert_eq!(deployments.categories, vec!["all"]);
        assert!(deployments.verbs.contains(&"create".to_string()));
        assert!(deployments.verbs.contains(&"watch".to_string()));

        let response_kind = deployments.response_kind.as_ref().unwrap();
        assert_eq!(response_kind.group, Some("apps".to_string()));
        assert_eq!(response_kind.version, Some("v1".to_string()));
        assert_eq!(response_kind.kind, Some("Deployment".to_string()));

        assert_eq!(deployments.subresources.len(), 2);
        let status_subresource = &deployments.subresources[0];
        assert_eq!(status_subresource.subresource, Some("status".to_string()));
    }

    #[test]
    fn deserialize_core_api_discovery() {
        // Sample response from /api with aggregated discovery (core group)
        let json = r#"{
            "kind": "APIGroupDiscoveryList",
            "apiVersion": "apidiscovery.k8s.io/v2",
            "metadata": {},
            "items": [
                {
                    "metadata": {
                        "name": ""
                    },
                    "versions": [
                        {
                            "version": "v1",
                            "freshness": "Current",
                            "resources": [
                                {
                                    "resource": "pods",
                                    "responseKind": {
                                        "group": "",
                                        "version": "v1",
                                        "kind": "Pod"
                                    },
                                    "scope": "Namespaced",
                                    "singularResource": "pod",
                                    "verbs": ["create", "delete", "deletecollection", "get", "list", "patch", "update", "watch"],
                                    "shortNames": ["po"],
                                    "categories": ["all"]
                                },
                                {
                                    "resource": "namespaces",
                                    "responseKind": {
                                        "group": "",
                                        "version": "v1",
                                        "kind": "Namespace"
                                    },
                                    "scope": "Cluster",
                                    "singularResource": "namespace",
                                    "verbs": ["create", "delete", "get", "list", "patch", "update", "watch"],
                                    "shortNames": ["ns"]
                                }
                            ]
                        }
                    ]
                }
            ]
        }"#;

        let result: APIGroupDiscoveryList = serde_json::from_str(json).unwrap();

        assert_eq!(result.items.len(), 1);
        let core_group = &result.items[0];

        // Core group has empty name
        assert_eq!(
            core_group.metadata.as_ref().and_then(|m| m.name.as_ref()),
            Some(&"".to_string())
        );

        let v1 = &core_group.versions[0];
        assert_eq!(v1.resources.len(), 2);

        // Check pods (namespaced)
        let pods = &v1.resources[0];
        assert_eq!(pods.resource, Some("pods".to_string()));
        assert_eq!(pods.scope, Some("Namespaced".to_string()));

        // Check namespaces (cluster-scoped)
        let namespaces = &v1.resources[1];
        assert_eq!(namespaces.resource, Some("namespaces".to_string()));
        assert_eq!(namespaces.scope, Some("Cluster".to_string()));
    }

    #[test]
    fn serialize_roundtrip() {
        let original = APIGroupDiscoveryList {
            metadata: None,
            items: vec![APIGroupDiscovery {
                metadata: Some(ObjectMeta {
                    name: Some("test".to_string()),
                    ..Default::default()
                }),
                versions: vec![APIVersionDiscovery {
                    version: Some("v1".to_string()),
                    freshness: Some("Current".to_string()),
                    resources: vec![],
                }],
            }],
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: APIGroupDiscoveryList = serde_json::from_str(&json).unwrap();

        assert_eq!(original, deserialized);
    }
}
