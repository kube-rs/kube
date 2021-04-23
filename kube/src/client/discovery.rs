use crate::{api::ApiResource, Client};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResourceList;
use std::{cmp::Reverse, collections::HashMap};

/// Resource scope
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum Scope {
    /// Objects are global
    Cluster,
    /// Each object lives in namespace.
    Namespaced,
}

/// Operations that are supported on the resource
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Operations {
    /// Object can be created
    pub create: bool,
    /// Single object can be queried
    pub get: bool,
    /// Multiple objects can be queried
    pub list: bool,
    /// A watch can be started
    pub watch: bool,
    /// A single object can be deleted
    pub delete: bool,
    /// Multiple objects can be deleted
    pub delete_collection: bool,
    /// Object can be updated
    pub update: bool,
    /// Object can be patched
    pub patch: bool,
    /// All other verbs
    pub other: Vec<String>,
}

impl Operations {
    /// Returns empty `Operations`
    pub fn empty() -> Self {
        Operations {
            create: false,
            get: false,
            list: false,
            watch: false,
            delete: false,
            delete_collection: false,
            update: false,
            patch: false,
            other: Vec::new(),
        }
    }
}
/// Contains additional, detailed information abount API resource
#[derive(Debug, Clone)]
pub struct ApiResourceExtras {
    /// Scope of the resource
    pub scope: Scope,
    /// Available subresources. Please note that returned ApiResources are not
    /// standalone resources. Their name will be of form `subresource_name`,
    /// not `resource_name/subresource_name`.
    /// To work with subresources, use `Request` methods.
    pub subresources: Vec<(ApiResource, ApiResourceExtras)>,
    /// Supported operations on this resource
    pub operations: Operations,
}

impl ApiResourceExtras {
    /// Creates ApiResourceExtras from `meta::v1::APIResourceList` instance.
    /// This function correctly sets all fields except `subresources`.
    /// # Panics
    /// Panics if list does not contain resource named `name`.
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
        let mut operations = Operations::empty();
        for verb in &ar.verbs {
            match verb.as_str() {
                "create" => operations.create = true,
                "get" => operations.get = true,
                "list" => operations.list = true,
                "watch" => operations.watch = true,
                "delete" => operations.delete = true,
                "deletecollection" => operations.delete_collection = true,
                "update" => operations.update = true,
                "patch" => operations.patch = true,
                _ => operations.other.push(verb.clone()),
            }
        }
        let mut subresources = Vec::new();
        let subresource_name_prefix = format!("{}/", name);
        for res in &list.resources {
            if let Some(subresource_name) = res.name.strip_prefix(&subresource_name_prefix) {
                let mut api_resource = ApiResource::from_apiresource(res, &list.group_version);
                api_resource.plural = subresource_name.to_string();
                let extra = ApiResourceExtras::from_apiresourcelist(list, &res.name);
                subresources.push((api_resource, extra));
            }
        }

        ApiResourceExtras {
            scope,
            subresources,
            operations,
        }
    }
}

struct GroupVersionData {
    version: String,
    // list: APIResourceList,
    resources: Vec<(ApiResource, ApiResourceExtras)>,
}

impl GroupVersionData {
    fn new(version: String, list: APIResourceList) -> Self {
        // TODO: could be better than O(N^2).
        let mut resources = Vec::new();
        for res in &list.resources {
            // skip subresources
            if res.name.contains('/') {
                continue;
            }
            let api_res = ApiResource::from_apiresource(res, &list.group_version);
            let extra = ApiResourceExtras::from_apiresourcelist(&list, &res.name);
            resources.push((api_res, extra));
        }
        GroupVersionData {
            version,
            resources,
            //list: list.clone(),
            // resources: filter_api_resource_list(list),
        }
    }
}

/// Describes one API group.
pub struct Group {
    name: String,
    versions_and_resources: Vec<GroupVersionData>,
    preferred_version: Option<String>,
}

/// High-level utility for runtime API discovery.
///
/// On creation `Discovery` queries Kubernetes API,
/// making list of all API resources, and provides a simple
/// interface on the top of that information.
///
/// # Resource representation
/// Each resource is represented as a pair
/// `(ApiResource, ApiResourceExtras)`. Former can be used
/// to make API requests (together with the `DynamicObject`
/// or `Object`). Latter provides additional information such
/// as scope or supported verbs.
pub struct Discovery {
    groups: HashMap<String, Group>,
}

// TODO: this is pretty unoptimized
impl Discovery {
    /// Discovers all APIs available in the cluster,
    /// including CustomResourceDefinitions
    // TODO: add more constructors
    #[tracing::instrument(skip(client))]
    pub async fn new(client: &Client) -> crate::Result<Self> {
        let api_groups = client.list_api_groups().await?;
        let mut groups = HashMap::new();
        for g in api_groups.groups {
            tracing::debug!(name = g.name.as_str(), "Listing group versions");
            if g.versions.is_empty() {
                tracing::warn!(name = g.name.as_str(), "Skipping group with empty versions list");
                continue;
            }
            let mut v = Vec::new();
            for vers in g.versions {
                let resource_list = client.list_api_group_resources(&vers.group_version).await?;

                v.push(GroupVersionData::new(vers.version, resource_list));
            }
            groups.insert(
                g.name.clone(),
                Group {
                    name: g.name,
                    versions_and_resources: v,
                    preferred_version: g.preferred_version.map(|v| v.version),
                },
            );
        }

        let coreapis = client.list_core_api_versions().await?;
        let mut core_v = Vec::new();
        for core_ver in coreapis.versions {
            let resource_list = client.list_core_api_resources(&core_ver).await?;
            core_v.push(GroupVersionData::new(core_ver, resource_list));
        }
        groups.insert(
            Group::CORE_GROUP.to_string(),
            Group {
                name: Group::CORE_GROUP.to_string(),
                versions_and_resources: core_v,
                preferred_version: Some("v1".to_string()),
            },
        );

        groups.values_mut().for_each(|group| group.sort_versions());

        Ok(Discovery { groups })
    }

    /// Utility function that splits apiVersion into a group and version
    /// that can be later used with this type.
    pub fn parse_api_version(api_version: &str) -> Option<(&str, &str)> {
        let mut iter = api_version.rsplitn(2, '/');
        let version = iter.next()?;
        let group = iter.next().unwrap_or(Group::CORE_GROUP);
        Some((group, version))
    }

    /// Returns iterator over all served groups
    pub fn groups(&self) -> impl Iterator<Item = &Group> {
        self.groups.iter().map(|(_, group)| group)
    }

    /// Returns information about the group `g`, if it is served.
    pub fn group(&self, g: &str) -> Option<&Group> {
        self.groups.get(g)
    }

    /// Checks if the group `g` is served.
    pub fn has_group(&self, g: &str) -> bool {
        self.group(g).is_some()
    }

    /// Returns resource with given group, version and kind.
    ///
    /// This function returns `ApiResource` which can be used together
    /// with `DynamicObject` and raw `ApiResourceExtras` value with additional information.
    pub fn resolve_group_version_kind(
        &self,
        group: &str,
        version: &str,
        kind: &str,
    ) -> Option<(ApiResource, ApiResourceExtras)> {
        // TODO: could be better than O(N)
        let group = self.group(group)?;
        group
            .resources_by_version(version)
            .into_iter()
            .find(|res| res.0.kind == kind)
    }
}

impl Group {
    /// Core group name
    pub const CORE_GROUP: &'static str = "core";

    /// Returns the name of this group.
    /// For core group (served at `/api`), returns "core" (also declared as
    /// `Group::CORE`).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Actually sets up order promised by `version`
    fn sort_versions(&mut self) {
        self.versions_and_resources
            .sort_by_cached_key(|ver_data| Version::parse(ver_data.version.as_str()))
    }

    /// Returns served versions (e.g. `["v1", "v2beta1"]`) of this group.
    /// This list is always non-empty, and sorted in the following order:
    /// - Stable versions (with the last being the first)
    /// - Beta versions (with the last being the first)
    /// - Alpha versions (with the last being the first)
    /// - Other versions, alphabetically
    // Order is documented here:
    // https://kubernetes.io/docs/tasks/extend-kubernetes/custom-resources/custom-resource-definition-versioning/#specify-multiple-versions
    pub fn versions(&self) -> impl Iterator<Item = &str> {
        let versions = self.versions_and_resources.as_slice();
        versions.iter().map(|ver_data| ver_data.version.as_str())
    }

    /// Returns preferred version for working with given group.
    pub fn preferred_version(&self) -> Option<&str> {
        self.preferred_version.as_deref()
    }

    /// Returns preferred version for working with given group.
    /// If server does not recommend one, this function picks
    /// "the most stable and the most recent" version.

    pub fn preferred_version_or_guess(&self) -> &str {
        match &self.preferred_version {
            Some(v) => v,
            None => self.versions().next().unwrap(),
        }
    }

    /// Returns resources available in version `ver` of this group.
    /// If the group does not support this version,
    /// returns empty vector.
    pub fn resources_by_version(&self, ver: &str) -> Vec<(ApiResource, ApiResourceExtras)> {
        let resources = self
            .versions_and_resources
            .iter()
            .find(|ver_data| ver_data.version == ver)
            .map(|ver_data| ver_data.resources.as_slice())
            .unwrap_or(&[]);
        resources.iter().cloned().collect()
    }
}

#[derive(PartialEq, Eq, Debug)]
enum Version {
    Stable(u32),
    Beta(u32, Option<u32>),
    Alpha(u32, Option<u32>),
    // CRDs and APIServices can use arbitrary strings as versions.
    Nonconformant(String),
}

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

    fn try_parse(v: &str) -> Option<Version> {
        let v = v.strip_prefix("v")?;
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

impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.to_sort_key().cmp(&other.to_sort_key())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::Version;
    fn check_parses_to(s: &str, v: Version) {
        assert_eq!(Version::parse(s), v);
    }

    #[test]
    fn test_stable() {
        check_parses_to("v1", Version::Stable(1));
        check_parses_to("v3", Version::Stable(3));
        check_parses_to("v10", Version::Stable(10));
    }

    #[test]
    fn test_prerelease() {
        check_parses_to("v1beta", Version::Beta(1, None));
        check_parses_to("v2alpha1", Version::Alpha(2, Some(1)));
        check_parses_to("v10beta12", Version::Beta(10, Some(12)));
    }

    fn check_not_parses(s: &str) {
        check_parses_to(s, Version::Nonconformant(s.to_string()))
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
}
