//! Abstractions on top of k8s_openapi::apimachinery::pkg::apis::meta::v1
use crate::{Error, Result, error::DiscoveryError};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};
use kube_core::{
    discovery::{
        ApiCapabilities, ApiResource, Scope,
        v2::{APIResourceDiscovery, APISubresourceDiscovery, APIVersionDiscovery},
    },
    gvk::{GroupVersion, ParseGroupVersionError},
};

/// Creates an `ApiResource` from a `meta::v1::APIResource` instance + its groupversion.
///
/// Returns a `DiscoveryError` if the passed group_version cannot be parsed
pub(crate) fn parse_apiresource(
    ar: &APIResource,
    group_version: &str,
) -> Result<ApiResource, ParseGroupVersionError> {
    let gv: GroupVersion = group_version.parse()?;
    // NB: not safe to use this with subresources (they don't have api_versions)
    Ok(ApiResource {
        group: ar.group.clone().unwrap_or_else(|| gv.group.clone()),
        version: ar.version.clone().unwrap_or_else(|| gv.version.clone()),
        api_version: gv.api_version(),
        kind: ar.kind.to_string(),
        plural: ar.name.clone(),
    })
}

/// Creates `ApiCapabilities` from a `meta::v1::APIResourceList` instance + a name from the list.
///
/// Returns a `DiscoveryError` if the list does not contain resource with passed `name`.
pub(crate) fn parse_apicapabilities(list: &APIResourceList, name: &str) -> Result<ApiCapabilities> {
    let ar = list
        .resources
        .iter()
        .find(|r| r.name == name)
        .ok_or_else(|| Error::Discovery(DiscoveryError::MissingResource(name.into())))?;
    let scope = if ar.namespaced {
        Scope::Namespaced
    } else {
        Scope::Cluster
    };

    let subresource_name_prefix = format!("{name}/");
    let mut subresources = vec![];
    for res in &list.resources {
        if let Some(subresource_name) = res.name.strip_prefix(&subresource_name_prefix) {
            let mut api_resource =
                parse_apiresource(res, &list.group_version).map_err(|ParseGroupVersionError(s)| {
                    Error::Discovery(DiscoveryError::InvalidGroupVersion(s))
                })?;
            api_resource.plural = subresource_name.to_string();
            let caps = parse_apicapabilities(list, &res.name)?; // NB: recursion
            subresources.push((api_resource, caps));
        }
    }
    Ok(ApiCapabilities {
        scope,
        subresources,
        operations: ar.verbs.clone(),
    })
}

/// Internal resource information and capabilities for a particular ApiGroup at a particular version
pub(crate) struct GroupVersionData {
    /// Pinned api version
    pub(crate) version: String,
    /// Pair of dynamic resource info along with what it supports.
    pub(crate) resources: Vec<(ApiResource, ApiCapabilities)>,
}

impl GroupVersionData {
    /// Given an APIResourceList, extract all information for a given version
    pub(crate) fn new(version: String, list: APIResourceList) -> Result<Self> {
        let mut resources = vec![];
        for res in &list.resources {
            // skip subresources
            if res.name.contains('/') {
                continue;
            }
            // NB: these two should be infallible from discovery when k8s api is well-behaved, but..
            let ar = parse_apiresource(res, &list.group_version).map_err(|ParseGroupVersionError(s)| {
                Error::Discovery(DiscoveryError::InvalidGroupVersion(s))
            })?;
            let caps = parse_apicapabilities(&list, &res.name)?;
            resources.push((ar, caps));
        }
        Ok(GroupVersionData { version, resources })
    }

    /// Create GroupVersionData from aggregated discovery v2 types
    /// Takes ownership of APIVersionDiscovery to avoid cloning.
    pub(crate) fn from_v2(group: &str, ver: APIVersionDiscovery) -> Self {
        let version = ver.version.unwrap_or_default();
        let gv = GroupVersion {
            group: group.to_string(),
            version: version.clone(),
        };

        let resources = ver
            .resources
            .into_iter()
            .map(|res| parse_v2_resource(res, &gv))
            .collect();

        GroupVersionData { version, resources }
    }
}

/// Convert an APIResourceDiscovery (v2) to ApiResource + ApiCapabilities
fn parse_v2_resource(res: APIResourceDiscovery, gv: &GroupVersion) -> (ApiResource, ApiCapabilities) {
    let kind = res.response_kind.and_then(|gvk| gvk.kind).unwrap_or_default();

    let scope = match res.scope.as_deref() {
        Some("Namespaced") => Scope::Namespaced,
        _ => Scope::Cluster,
    };

    let ar = ApiResource {
        group: gv.group.clone(),
        version: gv.version.clone(),
        api_version: gv.api_version(),
        kind,
        plural: res.resource.unwrap_or_default(),
    };

    let subresources = res
        .subresources
        .into_iter()
        .map(|sub| parse_v2_subresource(sub, gv, scope.clone()))
        .collect();

    let caps = ApiCapabilities {
        scope,
        subresources,
        operations: res.verbs,
    };

    (ar, caps)
}

/// Convert an APISubresourceDiscovery (v2) to ApiResource + ApiCapabilities
fn parse_v2_subresource(
    sub: APISubresourceDiscovery,
    gv: &GroupVersion,
    parent_scope: Scope,
) -> (ApiResource, ApiCapabilities) {
    let kind = sub.response_kind.and_then(|gvk| gvk.kind).unwrap_or_default();

    let ar = ApiResource {
        group: gv.group.clone(),
        version: gv.version.clone(),
        api_version: gv.api_version(),
        kind,
        plural: sub.subresource.unwrap_or_default(),
    };

    // Subresources inherit scope from parent resource
    let caps = ApiCapabilities {
        scope: parent_scope,
        subresources: vec![],
        operations: sub.verbs,
    };

    (ar, caps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kube_core::discovery::v2::GroupVersionKind;

    fn make_resource(resource: &str, kind: &str, scope: &str, verbs: Vec<&str>) -> APIResourceDiscovery {
        APIResourceDiscovery {
            resource: Some(resource.to_string()),
            response_kind: Some(GroupVersionKind {
                group: None,
                version: None,
                kind: Some(kind.to_string()),
            }),
            scope: Some(scope.to_string()),
            verbs: verbs.into_iter().map(String::from).collect(),
            subresources: vec![],
            ..Default::default()
        }
    }

    fn make_subresource(subresource: &str, kind: &str, verbs: Vec<&str>) -> APISubresourceDiscovery {
        APISubresourceDiscovery {
            subresource: Some(subresource.to_string()),
            response_kind: Some(GroupVersionKind {
                group: None,
                version: None,
                kind: Some(kind.to_string()),
            }),
            verbs: verbs.into_iter().map(String::from).collect(),
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_v2_resource_namespaced() {
        let res = make_resource("pods", "Pod", "Namespaced", vec![
            "get", "list", "watch", "create",
        ]);
        let gv = GroupVersion::gv("", "v1");

        let (ar, caps) = parse_v2_resource(res, &gv);

        assert_eq!(ar.group, "");
        assert_eq!(ar.version, "v1");
        assert_eq!(ar.api_version, "v1");
        assert_eq!(ar.kind, "Pod");
        assert_eq!(ar.plural, "pods");
        assert_eq!(caps.scope, Scope::Namespaced);
        assert_eq!(caps.operations, vec!["get", "list", "watch", "create"]);
        assert!(caps.subresources.is_empty());
    }

    #[test]
    fn test_parse_v2_resource_cluster_scoped() {
        let res = make_resource("nodes", "Node", "Cluster", vec!["get", "list"]);
        let gv = GroupVersion::gv("", "v1");

        let (ar, caps) = parse_v2_resource(res, &gv);

        assert_eq!(ar.kind, "Node");
        assert_eq!(caps.scope, Scope::Cluster);
    }

    #[test]
    fn test_parse_v2_resource_with_group() {
        let res = make_resource("deployments", "Deployment", "Namespaced", vec!["get", "list"]);
        let gv = GroupVersion::gv("apps", "v1");

        let (ar, caps) = parse_v2_resource(res, &gv);

        assert_eq!(ar.group, "apps");
        assert_eq!(ar.version, "v1");
        assert_eq!(ar.api_version, "apps/v1");
        assert_eq!(ar.kind, "Deployment");
        assert_eq!(ar.plural, "deployments");
        assert_eq!(caps.scope, Scope::Namespaced);
    }

    #[test]
    fn test_parse_v2_resource_with_subresources() {
        let mut res = make_resource("pods", "Pod", "Namespaced", vec!["get", "list"]);
        res.subresources = vec![
            make_subresource("status", "Pod", vec!["get", "patch"]),
            make_subresource("log", "Pod", vec!["get"]),
        ];
        let gv = GroupVersion::gv("", "v1");

        let (ar, caps) = parse_v2_resource(res, &gv);

        assert_eq!(ar.kind, "Pod");
        assert_eq!(caps.subresources.len(), 2);

        let (status_ar, status_caps) = &caps.subresources[0];
        assert_eq!(status_ar.plural, "status");
        assert_eq!(status_caps.scope, Scope::Namespaced); // inherited
        assert_eq!(status_caps.operations, vec!["get", "patch"]);

        let (log_ar, log_caps) = &caps.subresources[1];
        assert_eq!(log_ar.plural, "log");
        assert_eq!(log_caps.operations, vec!["get"]);
    }

    #[test]
    fn test_group_version_data_from_v2_core() {
        let ver = APIVersionDiscovery {
            version: Some("v1".to_string()),
            resources: vec![
                make_resource("pods", "Pod", "Namespaced", vec!["get", "list"]),
                make_resource("nodes", "Node", "Cluster", vec!["get", "list"]),
            ],
            freshness: Some("Current".to_string()),
        };

        let gvd = GroupVersionData::from_v2("", ver);

        assert_eq!(gvd.version, "v1");
        assert_eq!(gvd.resources.len(), 2);

        // Core group: api_version should be just "v1" (no group prefix)
        let (pod_ar, _) = &gvd.resources[0];
        assert_eq!(pod_ar.api_version, "v1");
        assert_eq!(pod_ar.group, "");
    }

    #[test]
    fn test_group_version_data_from_v2_apps() {
        let ver = APIVersionDiscovery {
            version: Some("v1".to_string()),
            resources: vec![make_resource("deployments", "Deployment", "Namespaced", vec![
                "get", "list", "create",
            ])],
            freshness: Some("Current".to_string()),
        };

        let gvd = GroupVersionData::from_v2("apps", ver);

        assert_eq!(gvd.version, "v1");
        assert_eq!(gvd.resources.len(), 1);

        let (ar, caps) = &gvd.resources[0];
        assert_eq!(ar.group, "apps");
        assert_eq!(ar.version, "v1");
        assert_eq!(ar.api_version, "apps/v1");
        assert_eq!(ar.kind, "Deployment");
        assert_eq!(caps.scope, Scope::Namespaced);
    }
}
