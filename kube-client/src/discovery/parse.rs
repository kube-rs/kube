//! Abstractions on top of k8s_openapi::apimachinery::pkg::apis::meta::v1
use crate::{error::DiscoveryError, Error, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};
use kube_core::{
    discovery::ApiResource,
    gvk::{GroupVersion, ParseGroupVersionError},
};

/// Creates an `ApiResource` from a `meta::v1::APIResource` instance + its groupversion.
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
        namespaced: ar.namespaced,
        verbs: ar.verbs.clone(),
        subresources: vec![],
    })
}

/// Scans nearby `meta::v1::APIResourceList` for subresources with a matching prefix
pub(crate) fn find_subresources(list: &APIResourceList, name: &str) -> Result<Vec<ApiResource>> {
    let subresource_name_prefix = format!("{}/", name);
    let mut subresources = vec![];
    for res in &list.resources {
        if let Some(subresource_name) = res.name.strip_prefix(&subresource_name_prefix) {
            let mut api_resource =
                parse_apiresource(res, &list.group_version).map_err(|ParseGroupVersionError(s)| {
                    Error::Discovery(DiscoveryError::InvalidGroupVersion(s))
                })?;
            api_resource.plural = subresource_name.to_string();
            subresources.push(api_resource);
        }
    }
    Ok(subresources)
}

/// Internal resource information and capabilities for a particular ApiGroup at a particular version
pub(crate) struct GroupVersionData {
    /// Pinned api version
    pub(crate) version: String,
    /// Pair of dynamic resource info along with what it supports.
    pub(crate) resources: Vec<ApiResource>,
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
            let mut ar =
                parse_apiresource(res, &list.group_version).map_err(|ParseGroupVersionError(s)| {
                    Error::Discovery(DiscoveryError::InvalidGroupVersion(s))
                })?;
            ar.subresources = find_subresources(&list, &res.name)?;
            resources.push(ar);
        }
        Ok(GroupVersionData { version, resources })
    }
}
