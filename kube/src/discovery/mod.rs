//! High-level utilities for runtime API discovery.

use crate::{Client, Error, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIGroup, APIResourceList, APIVersions};
pub use kube_core::dynamic::ApiResource;
use kube_core::gvk::{GroupVersion, GroupVersionKind};
use std::collections::HashMap;

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
///
/// Constructed internally during discovery.
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
    /// Creates ApiCapabilities from `meta::v1::APIResourceList` instance + a name from the list.
    ///
    /// Panics if list does not contain resource with passed `name`.
    fn from_apiresourcelist(list: &APIResourceList, name: &str) -> Self {
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
                #[allow(deprecated)] // will make this method not public later
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
            #[allow(deprecated)] // will make this method not public later
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

/// How the Discovery client decides what api groups to scan
enum DiscoveryMode {
    /// Only allow explicitly listed apigroups
    Allow(Vec<String>),
    /// Allow all apigroups except the ones listed
    Block(Vec<String>),
}

impl DiscoveryMode {
    #[allow(clippy::ptr_arg)] // hashmap complains on &str here
    fn is_queryable(&self, group: &String) -> bool {
        match &self {
            Self::Allow(allowed) => allowed.contains(group),
            Self::Block(blocked) => !blocked.contains(group),
        }
    }
}

/// A caching client for running API discovery against the Kubernetes API.
///
/// This simplifies the required querying and type matching, and stores the responses
/// for each discovered api group and exposes helpers to access them.
///
/// The discovery process varies in complexity depending on:
/// - how much you know about the kind(s) and group(s) you are interested in
/// - how many groups you are interested in
///
/// Discovery can be performed on:
/// - all api groups (default)
/// - a subset of api groups (by setting Discovery::filter)
///
/// To make use of discovered apis, extract one or more [`ApiGroup`]s from it,
/// or resolve a precise one using [`Discovery::resolve_gvk`](crate::discovery::Discovery::resolve_gvk).
///
/// If caching of results is __not required__, then a simpler [`Oneshot`](crate::discovery::Oneshot) discovery system can be used.
///
/// [`ApiGroup`]: crate::discovery::ApiGroup
pub struct Discovery {
    client: Client,
    groups: HashMap<String, ApiGroup>,
    mode: DiscoveryMode,
}

/// Caching discovery interface
///
/// Builds an internal map of its cache
impl Discovery {
    /// Construct a caching api discovery client
    pub fn new(client: Client) -> Self {
        let groups = HashMap::new();
        let mode = DiscoveryMode::Block(vec![]);
        Self { client, groups, mode }
    }

    /// Configure the discovery client to only look for the listed apigroups
    pub fn filter(mut self, allow: &[&str]) -> Self {
        self.mode = DiscoveryMode::Allow(allow.iter().map(ToString::to_string).collect());
        self
    }

    /// Runs or re-runs the configured discovery algorithm and updates/populates the cache
    ///
    /// The cache is empty cleared when this is started. By default, every api group found is checked,
    /// causing `N+2` queries to the api server (where `N` is number of api groups).
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery::{Discovery, verbs, Scope}, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let discovery = Discovery::new(client.clone()).run().await?;
    ///     for group in discovery.groups() {
    ///         for (ar, caps) in group.recommended_resources() {
    ///             if !caps.supports_operation(verbs::LIST) {
    ///                 continue;
    ///             }
    ///             let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///             // can now api.list() to emulate kubectl get all --all
    ///             for obj in api.list(&Default::default()).await? {
    ///                 println!("{} {}: {}", ar.api_version, ar.kind, obj.name());
    ///             }
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    /// See a bigger example in [examples/dynamic.api](https://github.com/clux/kube-rs/blob/master/examples/dynamic_api.rs)
    pub async fn run(mut self) -> Result<Self> {
        self.groups.clear();
        #[allow(deprecated)] // will make this method not public later
        let api_groups = self.client.list_api_groups().await?;
        // query regular groups + crds under /apis
        for g in api_groups.groups {
            tracing::debug!(name = g.name.as_str(), "Listing group versions");
            let key = g.name.clone();
            if self.mode.is_queryable(&key) {
                if let Some(apigroup) = ApiGroup::query(&self.client, g).await? {
                    self.groups.insert(key, apigroup);
                }
            }
        }
        // query core versions under /api
        let corekey = ApiGroup::CORE_GROUP.to_string();
        if self.mode.is_queryable(&corekey) {
            #[allow(deprecated)] // will make this method not public later
            let coreapis = self.client.list_core_api_versions().await?;
            if let Some(apigroup) = ApiGroup::query_core(&self.client, coreapis).await? {
                self.groups.insert(corekey, apigroup);
            }
        }
        Ok(self)
    }
}

/// Interface to the Discovery cache
impl Discovery {
    /// Returns iterator over all served groups
    pub fn groups(&self) -> impl Iterator<Item = &ApiGroup> {
        self.groups.values()
    }

    /// Returns the [`ApiGroup`] for a given group if served
    pub fn get(&self, group: &str) -> Option<&ApiGroup> {
        self.groups.get(group)
    }

    /// Check if a group is served by the apiserver
    pub fn has_group(&self, group: &str) -> bool {
        self.groups.contains_key(group)
    }

    /// Finds an [`ApiResource`] and its [`ApiCapabilities`] after discovery by matching a GVK
    ///
    /// This is for quick extraction after having done a complete discovery.
    /// If you are only interested in a single kind, consider [`Discovery::oneshot_gvk`](crate::Discovery::oneshot_gvk).
    pub fn resolve_gvk(&self, gvk: &GroupVersionKind) -> Option<(ApiResource, ApiCapabilities)> {
        self.get(&gvk.group)?
            .versioned_resources(&gvk.version)
            .into_iter()
            .find(|res| res.0.kind == gvk.kind)
    }
}

/// Oneshot discovery
///
/// The oneshot system will return specific information for:
/// - a single group like "apiregistration.k8s.io" via [`Oneshot::group`]
/// - a single group at a particular version: e.g. "apiregistration.k8s.io/v1" via [`Oneshot::gv`]
/// - a particular kind in a group at a particular version via [`Oneshot::gvk`]
///
/// [`Oneshot::group`]: crate::discovery::Oneshot::group
/// [`Oneshot::gv`]: crate::discovery::Oneshot::gv
/// [`Oneshot::gvk`]: crate::discovery::Oneshot::gvk
pub struct Oneshot {}

/// Oneshot discovery helpers
///
/// These do not return the usual Discovery type, but instead more precise types depending on what
/// was asked for. More ergonomic when you know what you want.
impl Oneshot {
    /// Discovers all APIs available under a certain group and return the singular ApiGroup
    ///
    /// This is recommended if you work with one group, but do not want to pin the version
    /// of the apigroup. Instead you will work with a recommended version (preferred or latest).
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let apigroup = discovery::Oneshot::group(&client, "apiregistration.k8s.io").await?;
    ///     let (ar, caps) = apigroup.recommended_kind("APIService").unwrap();
    ///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///     for service in api.list(&Default::default()).await? {
    ///         println!("Found APIService: {}", service.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn group(client: &Client, apigroup: &str) -> Result<ApiGroup> {
        if apigroup == ApiGroup::CORE_GROUP {
            #[allow(deprecated)] // will make this method not public later
            let coreapis = client.list_core_api_versions().await?;
            if let Some(apigroup) = ApiGroup::query_core(&client, coreapis).await? {
                return Ok(apigroup);
            }
        } else {
            #[allow(deprecated)] // will make this method not public later
            let api_groups = client.list_api_groups().await?;
            for g in api_groups.groups {
                if g.name != apigroup {
                    continue;
                }
                if let Some(apigroup) = ApiGroup::query(&client, g).await? {
                    return Ok(apigroup);
                }
            }
        }
        Err(Error::MissingApiGroup(apigroup.to_string()))
    }

    /// Discovers all APIs available under a certain group at a particular version and return the singular ApiGroup
    ///
    /// This is a cheaper variant of `Discovery::oneshot` when you know what version you want.
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let gv = "apiregistration.k8s.io/v1".parse()?;
    ///     let apigroup = discovery::Oneshot::gv(&client, &gv).await?;
    ///     let (ar, caps) = apigroup.recommended_kind("APIService").unwrap();
    ///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///     for service in api.list(&Default::default()).await? {
    ///         println!("Found APIService: {}", service.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// While this example only uses a single kind, this type of discovery works best when you need more
    /// than a single `kind`.
    /// If you only need a single `kind`, `Discovery::oneshot_gvk` is the best solution.
    pub async fn gv(client: &Client, gv: &GroupVersion) -> Result<ApiGroup> {
        ApiGroup::query_gv(&client, gv).await
    }

    /// Single discovery for a single GVK
    ///
    /// This is an optimized function that avoids the unnecessary listing of api groups.
    /// It merely requests the api group resources for the specified apigroup, and then resolves the kind.
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject, GroupVersionKind}, discovery, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let gvk = GroupVersionKind::gvk("apiregistration.k8s.io", "v1", "APIService");
    ///     let (ar, caps) = discovery::Oneshot::gvk(&client, &gvk).await?;
    ///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///     for service in api.list(&Default::default()).await? {
    ///         println!("Found APIService: {}", service.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn gvk(client: &Client, gvk: &GroupVersionKind) -> Result<(ApiResource, ApiCapabilities)> {
        ApiGroup::query_gvk(client, &gvk).await
    }
}
// ----------------------------------------------------------------------------
// ApiGroup
// ----------------------------------------------------------------------------

/// Describes one API groups collected resources and capabilities.
///
/// Each `ApiGroup` contains all data pinned to a each version.
/// In particular, one data set within the `ApiGroup` for `"apiregistration.k8s.io"`
/// is the subset pinned to `"v1"`; commonly referred to as `"apiregistration.k8s.io/v1"`.
///
/// If you know the version of the discovered group, you can fetch it directly:
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), kube::Error> {
///     let client = Client::try_default().await?;
///     let apigroup = discovery::Oneshot::group(&client, "apiregistration.k8s.io").await?;
///      for (apiresource, caps) in apigroup.versioned_resources("v1") {
///          println!("Found ApiResource {}", apiresource.kind);
///      }
///     Ok(())
/// }
/// ```
///
/// But if you do not know this information, you can use [`ApiGroup::preferred_version_or_latest`].
///
/// Whichever way you choose the end result is something describing a resource and its abilities:
/// - `Vec<(ApiResource, `ApiCapabilities)>` :: for all resources in a versioned ApiGroup
/// - `(ApiResource, ApiCapabilities)` :: for a single kind under a versioned ApiGroud
///
/// These two types: [`ApiResource`], and [`ApiCapabilities`]
/// should contain the information needed to construct an [`Api`](crate::Api) and start querying the kubernetes API.
/// You will likely need to use [`DynamicObject`] as the generic type for Api to do this,
/// as well as the [`ApiResource`] for the `DynamicType` for the [`Resource`] trait.
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), kube::Error> {
///     let client = Client::try_default().await?;
///     let apigroup = discovery::Oneshot::group(&client, "apiregistration.k8s.io").await?;
///     let (ar, caps) = apigroup.recommended_kind("APIService").unwrap();
///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
///     for service in api.list(&Default::default()).await? {
///         println!("Found APIService: {}", service.name());
///     }
///     Ok(())
/// }
/// ```
/// [`ApiResource`]: crate::discovery::ApiResource
/// [`ApiCapabilities`]: crate::discovery::ApiCapabilities
/// [`DynamicObject`]: crate::api::DynamicObject
/// [`Resource`]: crate::Resource
/// [`ApiGroup::preferred_version_or_latest`]: crate::discovery::ApiGroup::preferred_version_or_latest
/// [`ApiGroup::versioned_resources`]: crate::discovery::ApiGroup::versioned_resources
/// [`ApiGroup::recommended_resources`]: crate::discovery::ApiGroup::recommended_resources
/// [`ApiGroup::recommended_kind`]: crate::discovery::ApiGroup::recommended_kind
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
            #[allow(deprecated)] // will make this method not public later
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
            #[allow(deprecated)] // will make this method not public later
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
        self.data
            .sort_by_cached_key(|gvd| Version::parse(gvd.version.as_str()))
    }

    // shortcut method to give cheapest return for a single GVK
    async fn query_gvk(client: &Client, gvk: &GroupVersionKind) -> Result<(ApiResource, ApiCapabilities)> {
        let apiver = gvk.api_version();
        #[allow(deprecated)] // will make these method not public later
        let list = if gvk.group.is_empty() {
            client.list_core_api_resources(&apiver).await?
        } else {
            client.list_api_group_resources(&apiver).await?
        };
        for res in &list.resources {
            if res.kind == gvk.kind && !res.name.contains('/') {
                #[allow(deprecated)] // will make this method not public later
                let ar = ApiResource::from_apiresource(res, &list.group_version);
                let caps = ApiCapabilities::from_apiresourcelist(&list, &res.name);
                return Ok((ar, caps));
            }
        }
        Err(Error::MissingGVK(format!("{:?}", gvk)))
    }

    // shortcut method to give cheapest return for a pinned group
    async fn query_gv(client: &Client, gv: &GroupVersion) -> Result<Self> {
        let apiver = gv.api_version();
        #[allow(deprecated)] // will make these methods not public later
        let list = if gv.group.is_empty() {
            client.list_core_api_resources(&apiver).await?
        } else {
            client.list_api_group_resources(&apiver).await?
        };
        let data = GroupVersionData::new(gv.version.clone(), list);
        let group = ApiGroup {
            name: gv.group.clone(),
            data: vec![data],
            preferred: Some(gv.version.clone()), // you preferred what you asked for
        };
        Ok(group)
    }
}

/// Public ApiGroup interface
impl ApiGroup {
    /// Core group name
    pub const CORE_GROUP: &'static str = "";

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
        self.preferred
            .as_deref()
            .unwrap_or_else(|| self.versions().next().unwrap())
    }

    /// Returns the resources in the group at an arbitrary version string.
    ///
    /// If the group does not support this version, the returned vector is empty.
    ///
    /// If you are looking for the api recommended list of resources, or just on particular kind
    /// consider [`ApiGroup::recommended_resources`] or [`ApiGroup::recommended_kind`] instead.
    pub fn versioned_resources(&self, ver: &str) -> Vec<(ApiResource, ApiCapabilities)> {
        self.data
            .iter()
            .find(|gvd| gvd.version == ver)
            .map(|gvd| gvd.resources.clone())
            .unwrap_or_default()
    }

    /// Returns the recommended (preferred or latest) versioned resources in the group
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery::{self, verbs}, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let apigroup = discovery::Oneshot::group(&client, "apiregistration.k8s.io").await?;
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
    ///
    /// This is equivalent to taking the [`ApiGroup::versioned_resources`] at the [`ApiGroup::preferred_version_or_latest`].
    pub fn recommended_resources(&self) -> Vec<(ApiResource, ApiCapabilities)> {
        let ver = self.preferred_version_or_latest();
        self.versioned_resources(ver)
    }

    /// Returns the recommended version of the `kind` in the recommended resources (if found)
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let apigroup = discovery::Oneshot::group(&client, "apiregistration.k8s.io").await?;
    ///     let (ar, caps) = apigroup.recommended_kind("APIService").unwrap();
    ///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///     for service in api.list(&Default::default()).await? {
    ///         println!("Found APIService: {}", service.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    ///
    /// This is equivalent to filtering the [`ApiGroup::versioned_resources`] at [`ApiGroup::preferred_version_or_latest`] against a chosen `kind`.
    pub fn recommended_kind(&self, kind: &str) -> Option<(ApiResource, ApiCapabilities)> {
        let ver = self.preferred_version_or_latest();
        for (ar, caps) in self.versioned_resources(ver) {
            if ar.kind == kind {
                return Some((ar, caps));
            }
        }
        None
    }
}

// an implementation of mentioned kubernetes version priority
mod version;
pub(crate) use version::Version;
