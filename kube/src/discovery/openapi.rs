//! Abstractions on top of k8s_openapi::apimachinery::pkg::apis::meta::v1
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};
use kube_core::{
    discovery::{ApiCapabilities, ApiResource, Scope},
    gvk::GroupVersion,
};

// TODO: create actual errors from the assumptions here

/// Creates an `ApiResource` from a `meta::v1::APIResource` instance + its groupversion.
pub(crate) fn parse_apiresource(ar: &APIResource, group_version: &str) -> ApiResource {
    let gv: GroupVersion = group_version.parse().expect("valid group_version");
    // NB: not safe to use this with subresources (they don't have api_versions)
    ApiResource {
        group: ar.group.clone().unwrap_or_else(|| gv.group.clone()),
        version: ar.version.clone().unwrap_or_else(|| gv.version.clone()),
        api_version: gv.api_version(),
        kind: ar.kind.to_string(),
        plural: ar.name.clone(),
    }
}

/// Creates `ApiCapabilities` from a `meta::v1::APIResourceList` instance + a name from the list.
///
/// Panics if list does not contain resource with passed `name`.
pub(crate) fn parse_apicapabilities(list: &APIResourceList, name: &str) -> ApiCapabilities {
    let ar = list
        .resources
        .iter()
        .find(|r| r.name == name)
        .expect("resource not found in APIResourceList");
    let scope = if ar.namespaced {
        Scope::Namespaced
    } else {
        Scope::Cluster
    };

    let subresource_name_prefix = format!("{}/", name);
    let mut subresources = vec![];
    for res in &list.resources {
        if let Some(subresource_name) = res.name.strip_prefix(&subresource_name_prefix) {
            let mut api_resource = parse_apiresource(res, &list.group_version);
            api_resource.plural = subresource_name.to_string();
            let caps = parse_apicapabilities(list, &res.name); // NB: recursion
            subresources.push((api_resource, caps));
        }
    }
    ApiCapabilities {
        scope,
        subresources,
        operations: ar.verbs.clone(),
    }
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
    pub(crate) fn new(version: String, list: APIResourceList) -> Self {
        let mut resources = vec![];
        for res in &list.resources {
            // skip subresources
            if res.name.contains('/') {
                continue;
            }
            let ar = parse_apiresource(res, &list.group_version);
            let caps = parse_apicapabilities(&list, &res.name);
            resources.push((ar, caps));
        }
        GroupVersionData { version, resources }
    }
}
