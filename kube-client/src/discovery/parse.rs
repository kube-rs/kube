//! Abstractions on top of k8s_openapi::apimachinery::pkg::apis::meta::v1
use crate::{error::DiscoveryError, Error, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};
use kube_core::{
    discovery::{ApiCapabilities, ApiResource, Scope},
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

    let subresource_name_prefix = format!("{}/", name);
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
}
