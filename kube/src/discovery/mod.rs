//! High-level utilities for runtime API discovery.

use crate::{Client, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIGroup, APIResourceList, APIVersions};
use kube_core::{gvk::GroupVersionKind, dynamic::ApiResource};
use std::{cmp::Reverse, collections::HashMap};

/// Resource scope
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Scope {
    /// Objects are global
    Cluster,
    /// Each object lives in namespace.
    Namespaced,
}

/// Defines standard verbs
pub mod verbs {
    /// Create a resource
    pub const CREATE: &str = "create";
    /// Get single resource
    pub const GET: &str = "get";
    /// List objects
    pub const LIST: &str = "list";
    /// Watch for objects changes
    pub const WATCH: &str = "watch";
    /// Delete single object
    pub const DELETE: &str = "delete";
    /// Delete multiple objects at once
    pub const DELETE_COLLECTION: &str = "deletecollection";
    /// Update an object
    pub const UPDATE: &str = "update";
    /// Patch an object
    pub const PATCH: &str = "patch";
}

/// Contains additional, detailed information abount API resource
#[derive(Debug, Clone)]
pub struct ApiCapabilities {
    /// Scope of the resource
    pub scope: Scope,
    /// Available subresources. Please note that returned ApiResources are not
    /// standalone resources. Their name will be of form `subresource_name`,
    /// not `resource_name/subresource_name`.
    /// To work with subresources, use `Request` methods.
    pub subresources: Vec<(ApiResource, ApiCapabilities)>,
    /// Supported operations on this resource
    pub operations: Vec<String>,
}

impl ApiCapabilities {
    /// Creates ApiCapabilities from `meta::v1::APIResourceList` instance.
    /// This function correctly sets all fields except `subresources`.
    /// # Panics
    /// Panics if list does not contain resource with passed `name`.
    pub fn from_apiresourcelist(list: &APIResourceList, name: &str) -> Self {
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
                let mut api_resource = ApiResource::from_apiresource(res, &list.group_version);
                api_resource.plural = subresource_name.to_string();
                let extra = ApiCapabilities::from_apiresourcelist(list, &res.name);
                subresources.push((api_resource, extra));
            }
        }
        Self {
            scope,
            subresources,
            operations: ar.verbs.clone(),
        }
    }

    /// Checks that given verb is supported on this resource.
    pub fn supports_operation(&self, operation: &str) -> bool {
        self.operations.iter().any(|op| op == operation)
    }
}

/// Resource information and capabilities for a particular ApiGroup at a particular version
struct GroupVersionData {
    /// Pinned api version
    version: String,
    /// Pair of dynamic resource info along with what it supports.
    resources: Vec<(ApiResource, ApiCapabilities)>,
}

impl GroupVersionData {
    fn new(version: String, list: APIResourceList) -> Self {
        let mut resources = vec![];
        for res in &list.resources {
            // skip subresources
            if res.name.contains('/') {
                continue;
            }
            let ar = ApiResource::from_apiresource(res, &list.group_version);
            let extra = ApiCapabilities::from_apiresourcelist(&list, &res.name);
            resources.push((ar, extra));
        }
        GroupVersionData { version, resources }
    }
}

// ----------------------------------------------------------------------------
// Discovery
// ----------------------------------------------------------------------------

/// A client helper for running API discovery against the Kubernetes API.
///
/// It simplifies the various querying and type matching to ensure that we have
/// a sensible version to use from each api group and resource.
///
/// Discovery can be limited to:
/// - a single group such as "apiregistration.k8s.io" (via `Discovery::single`)
/// - multiple named groups (via repeat calls to `Discovery::single`)
///
/// Or you can use it to discover everything via `Discovery::all`.
/// Internally, it will use a series of `Client` calls (one per group) to discover what was requested.
pub struct Discovery {
    groups: HashMap<String, ApiGroup>,
}

impl Discovery {
    /// Discovers all APIs available in the cluster including CustomResourceDefinitions
    pub async fn all(client: &Client) -> Result<Self> {
        let api_groups = client.list_api_groups().await?;
        let mut groups = HashMap::new();
        // query regular groups under /apis
        for g in api_groups.groups {
            tracing::debug!(name = g.name.as_str(), "Listing group versions");
            if let Some(apigroup) = ApiGroup::query(&client, g).await? {
                groups.insert(apigroup.name.clone(), apigroup);
            }
        }
        // query core versions under /api
        let coreapis = client.list_core_api_versions().await?;
        if let Some(apigroup) = ApiGroup::query_core(&client, coreapis).await? {
            groups.insert(ApiGroup::CORE_GROUP.to_string(), apigroup);
        }
        Ok(Discovery { groups })
    }

    /// Discovers all APIs available under a certain group and return the singular ApiGroup
    ///
    /// You can safely unwrap the Option if you know the apigroup passed exists on the apiserver.
    pub async fn single(client: &Client, apigroup: &str) -> Result<Option<ApiGroup>> {
        let api_groups = client.list_api_groups().await?;
        for g in api_groups.groups {
            if g.name != apigroup {
                continue;
            }
            if let Some(apigroup) = ApiGroup::query(&client, g).await? {
                return Ok(Some(apigroup));
            }
        }
        Ok(None)
    }

    // make something in between? vector of group inputs? could just call `Discovery::single` again..
}


//TODO: make a helper to create a GVK from this + kind to allow resolve_gvk to be easier
//pub fn parse_api_version(api_version: &str) -> Option<(&str, &str)> {
//    let mut iter = api_version.rsplitn(2, '/');
//    let version = iter.next()?;
//    let group = iter.next().unwrap_or(ApiGroup::CORE_GROUP);
//    Some((group, version))
//}

/// Public query interface
impl Discovery {
    /// Returns iterator over all served groups
    pub fn groups(&self) -> impl Iterator<Item = &ApiGroup> {
        self.groups.values()
    }

    /// Returns the `ApiGroup` for a given group if served
    pub fn get(&self, group: &str) -> Option<&ApiGroup> {
        self.groups.get(group)
    }

    /// Check if a group is served by the apiserver
    pub fn has_group(&self, group: &str) -> bool {
        self.groups.contains_key(group)
    }

    /// Finds an ApiResource and its ApiCapabilities after discovery by matching a GVK
    pub fn resolve_gvk(&self, gvk: &GroupVersionKind) -> Option<(ApiResource, ApiCapabilities)> {
        self.get(&gvk.group)?
            .resources_by_version(&gvk.version)
            .into_iter()
            .find(|res| res.0.kind == gvk.kind)
    }
}

// ----------------------------------------------------------------------------
// ApiGroup
// ----------------------------------------------------------------------------

/// Describes one API groups collected resources and capabilities.
pub struct ApiGroup {
    /// Name of the group e.g. apiregistration.k8s.io
    name: String,
    /// List of resource information, capabilities at particular versions
    data: Vec<GroupVersionData>,
    /// Preferred version if exported by the `APIGroup`
    preferred: Option<String>,
}

/// Internal queriers to convert from an APIGroup (or APIVersions for core) to our ApiGroup
///
/// These queriers ignore groups with empty versions.
/// This ensures that `ApiGroup::preferred_version_or_latest` always have an answer.
/// On construction, they also sort the internal vec of GroupVersionData according to `Version`.
impl ApiGroup {
    async fn query(client: &Client, g: APIGroup) -> Result<Option<Self>> {
        tracing::debug!(name = g.name.as_str(), "Listing group versions");
        if g.versions.is_empty() {
            tracing::warn!(name = g.name.as_str(), "Skipping group with empty versions list");
            return Ok(None);
        }
        let mut data = vec![];
        for vers in &g.versions {
            let resources = client.list_api_group_resources(&vers.group_version).await?;
            data.push(GroupVersionData::new(vers.version.clone(), resources));
        }
        let mut group = ApiGroup {
            name: g.name,
            data,
            preferred: g.preferred_version.map(|v| v.version),
        };
        group.sort_versions();
        Ok(Some(group))
    }
    async fn query_core(client: &Client, coreapis: APIVersions) -> Result<Option<Self>> {
        let mut data = vec![];
        if coreapis.versions.is_empty() {
            tracing::warn!("Skipping core group with empty versions list");
            return Ok(None);
        }
        for v in coreapis.versions {
            let resources = client.list_core_api_resources(&v).await?;
            data.push(GroupVersionData::new(v, resources));
        }
        let mut group = ApiGroup {
            name: ApiGroup::CORE_GROUP.to_string(),
            data,
            preferred: Some("v1".to_string()),
        };
        group.sort_versions();
        Ok(Some(group))
    }
    fn sort_versions(&mut self) {
        self.data.sort_by_cached_key(|gvd| Version::parse(gvd.version.as_str()))
    }
}

/// Public ApiGroup interface
impl ApiGroup {
    /// Core group name
    pub const CORE_GROUP: &'static str = "core";

    /// Returns the name of this group.
    ///
    /// For the core group (served at `/api`), it returns `ApiGroup::CORE`.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns served versions (e.g. `["v1", "v2beta1"]`) of this group.
    ///
    /// This list is always non-empty, and sorted in the following order:
    /// - Stable versions (with the last being the first)
    /// - Beta versions (with the last being the first)
    /// - Alpha versions (with the last being the first)
    /// - Other versions, alphabetically
    ///
    /// in accordance with [kubernetes version priority](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority).
    pub fn versions(&self) -> impl Iterator<Item = &str> {
        self.data.as_slice().iter().map(|gvd| gvd.version.as_str())
    }

    /// Returns preferred version for working with given group.
    pub fn preferred_version(&self) -> Option<&str> {
        self.preferred.as_deref()
    }

    /// Returns the preferred version or latest version for working with given group.
    ///
    /// If server does not recommend one, we pick the "most stable and most recent" version
    /// in accordance with [kubernetes version priority](https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#version-priority).
    pub fn preferred_version_or_latest(&self) -> &str {
        // NB: self.versions is non-empty by construction in ApiGroup
        self.preferred.as_deref().unwrap_or_else(|| self.versions().next().unwrap())
    }

    /// Returns resources available by a version
    ///
    /// If the group does not support this version, the returned vector is empty.
    ///
    /// If you are looking for the api recommended list of resources, or just on particular kind
    /// consider `ApiGroup::recommended_resources` or `ApiGroup::recommended_kind` instead.
    pub fn resources_by_version(&self, ver: &str) -> Vec<(ApiResource, ApiCapabilities)> {
        self
            .data
            .iter()
            .find(|gvd| gvd.version == ver)
            .map(|gvd| gvd.resources.as_slice())
            .unwrap_or(&[])
            .to_vec()
    }

    /// Returns the recommended (preferred or latest versioned) resources in the ApiGroup
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery::{Discovery, verbs}, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let apigroup = Discovery::single(&client, "apiregistration.k8s.io").await?.unwrap();
    ///     for (ar, caps) in apigroup.recommended_resources() {
    ///         if !caps.supports_operation(verbs::LIST) {
    ///             continue;
    ///         }
    ///         let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///         for inst in api.list(&Default::default()).await? {
    ///             println!("Found {}: {}", ar.kind, inst.name());
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn recommended_resources(&self) -> Vec<(ApiResource, ApiCapabilities)> {
        let ver = self.preferred_version_or_latest();
        self.resources_by_version(ver)
    }

    /// Returns the recommended version of the Kind in the recommended resources (if found)
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, Discovery, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let apigroup = Discovery::single(&client, "apiregistration.k8s.io").await?.unwrap();
    ///     let (ar, caps) = apigroup.recommended_kind("APIService").unwrap();
    ///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///     for service in api.list(&Default::default()).await? {
    ///         println!("Found APIService: {}", service.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub fn recommended_kind(&self, kind: &str) -> Option<(ApiResource, ApiCapabilities)> {
        let ver = self.preferred_version_or_latest();
        for (ar, caps) in self.resources_by_version(ver) {
            if ar.kind == kind {
                return Some((ar, caps))
            }
        }
        None
    }
}

// ----------------------------------------------------------------------------
// Version definition to allow sorting (not exported)
// ----------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
enum Version {
    Stable(u32),
    Beta(u32, Option<u32>),
    Alpha(u32, Option<u32>),
    // CRDs and APIServices can use arbitrary strings as versions.
    Nonconformant(String),
}

impl Version {
    fn try_parse(v: &str) -> Option<Version> {
        let v = v.strip_prefix('v')?;
        let major_chars = v.chars().take_while(|ch| ch.is_ascii_digit()).count();
        let major = &v[..major_chars];
        let major: u32 = major.parse().ok()?;
        let v = &v[major_chars..];
        if v.is_empty() {
            return Some(Version::Stable(major));
        }
        if let Some(suf) = v.strip_prefix("alpha") {
            return if suf.is_empty() {
                Some(Version::Alpha(major, None))
            } else {
                Some(Version::Alpha(major, Some(suf.parse().ok()?)))
            };
        }
        if let Some(suf) = v.strip_prefix("beta") {
            return if suf.is_empty() {
                Some(Version::Beta(major, None))
            } else {
                Some(Version::Beta(major, Some(suf.parse().ok()?)))
            };
        }
        None
    }

    fn parse(v: &str) -> Version {
        match Self::try_parse(v) {
            Some(ver) => ver,
            None => Version::Nonconformant(v.to_string()),
        }
    }
}
// A key used to allow sorting Versions
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum VersionSortKey<'a> {
    Stable(Reverse<u32>),
    Beta(Reverse<u32>, Reverse<Option<u32>>),
    Alpha(Reverse<u32>, Reverse<Option<u32>>),
    Nonconformant(&'a str),
}
impl Version {
    fn to_sort_key(&self) -> VersionSortKey {
        match self {
            Version::Stable(v) => VersionSortKey::Stable(Reverse(*v)),
            Version::Beta(v, beta) => VersionSortKey::Beta(Reverse(*v), Reverse(*beta)),
            Version::Alpha(v, alpha) => VersionSortKey::Alpha(Reverse(*v), Reverse(*alpha)),
            Version::Nonconformant(nc) => VersionSortKey::Nonconformant(nc),
        }
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_sort_key().cmp(&other.to_sort_key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&other))
    }
}

#[cfg(test)]
mod tests {
    use super::Version;

    #[test]
    fn test_stable() {
        assert_eq!(Version::parse("v1"), Version::Stable(1));
        assert_eq!(Version::parse("v3"), Version::Stable(3));
        assert_eq!(Version::parse("v10"), Version::Stable(10));
    }

    #[test]
    fn test_prerelease() {
        assert_eq!(Version::parse("v1beta"), Version::Beta(1, None));
        assert_eq!(Version::parse("v2alpha1"), Version::Alpha(2, Some(1)));
        assert_eq!(Version::parse("v10beta12"), Version::Beta(10, Some(12)));
    }

    fn check_not_parses(s: &str) {
        assert_eq!(Version::parse(s), Version::Nonconformant(s.to_string()))
    }

    #[test]
    fn test_nonconformant() {
        check_not_parses("");
        check_not_parses("foo");
        check_not_parses("v");
        check_not_parses("v-1");
        check_not_parses("valpha");
        check_not_parses("vbeta3");
        check_not_parses("vv1");
        check_not_parses("v1alpha1hi");
        check_not_parses("v1zeta3");
    }

    #[test]
    fn test_version_ord() {
        // NB: semantically this is exact opposite of what makes sense, but we never export Version.
        // We can reverse the Ord, but would have to remember to .reverse() again after sorts.
        assert!(Version::Stable(2) < Version::Stable(1));
        assert!(Version::Stable(2) < Version::Stable(1));
        assert!(Version::Stable(1) < Version::Beta(1, None));
        assert!(Version::Stable(1) < Version::Beta(2, None));
        assert!(Version::Stable(2) < Version::Alpha(1, Some(2)));
        assert!(Version::Stable(1) < Version::Alpha(2, Some(2)));
        assert!(Version::Beta(1, None) < Version::Nonconformant("hi".into()));

        // The sorting results is what we export, and this works by default because Ord is reversed:
        let mut vers = vec![
            Version::Beta(2, Some(2)),
            Version::Stable(1),
            Version::Nonconformant("hi".into()),
            Version::Alpha(1, Some(2)),
            Version::Stable(2),
            Version::Beta(2, Some(3)),
        ];
        vers.sort();
        assert_eq!(vers, vec![
            Version::Stable(2),
            Version::Stable(1),
            Version::Beta(2, Some(3)),
            Version::Beta(2, Some(2)),
            Version::Alpha(1, Some(2)),
            Version::Nonconformant("hi".into()),
        ]);
    }
}
