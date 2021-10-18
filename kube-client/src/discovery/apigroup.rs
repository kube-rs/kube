use super::{
    parse::{self, GroupVersionData},
    version::Version,
};
use crate::{error::DiscoveryError, Client, Result};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIGroup, APIVersions};
pub use kube_core::discovery::{verbs, ApiCapabilities, ApiResource, Scope};
use kube_core::gvk::{GroupVersion, GroupVersionKind};


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
///     let apigroup = discovery::group(&client, "apiregistration.k8s.io").await?;
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
///     let apigroup = discovery::group(&client, "apiregistration.k8s.io").await?;
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
    pub(crate) async fn query_apis(client: &Client, g: APIGroup) -> Result<Self> {
        tracing::debug!(name = g.name.as_str(), "Listing group versions");
        let key = g.name;
        if g.versions.is_empty() {
            return Err(DiscoveryError::EmptyApiGroup(key).into());
        }
        let mut data = vec![];
        for vers in &g.versions {
            let resources = client.list_api_group_resources(&vers.group_version).await?;
            data.push(GroupVersionData::new(vers.version.clone(), resources)?);
        }
        let mut group = ApiGroup {
            name: key,
            data,
            preferred: g.preferred_version.map(|v| v.version),
        };
        group.sort_versions();
        Ok(group)
    }

    pub(crate) async fn query_core(client: &Client, coreapis: APIVersions) -> Result<Self> {
        let mut data = vec![];
        let key = ApiGroup::CORE_GROUP.to_string();
        if coreapis.versions.is_empty() {
            return Err(DiscoveryError::EmptyApiGroup(key).into());
        }
        for v in coreapis.versions {
            let resources = client.list_core_api_resources(&v).await?;
            data.push(GroupVersionData::new(v, resources)?);
        }
        let mut group = ApiGroup {
            name: ApiGroup::CORE_GROUP.to_string(),
            data,
            preferred: Some("v1".to_string()),
        };
        group.sort_versions();
        Ok(group)
    }

    fn sort_versions(&mut self) {
        self.data
            .sort_by_cached_key(|gvd| Version::parse(gvd.version.as_str()))
    }

    // shortcut method to give cheapest return for a single GVK
    pub(crate) async fn query_gvk(
        client: &Client,
        gvk: &GroupVersionKind,
    ) -> Result<(ApiResource, ApiCapabilities)> {
        let apiver = gvk.api_version();
        let list = if gvk.group.is_empty() {
            client.list_core_api_resources(&apiver).await?
        } else {
            client.list_api_group_resources(&apiver).await?
        };
        for res in &list.resources {
            if res.kind == gvk.kind && !res.name.contains('/') {
                let ar = parse::parse_apiresource(res, &list.group_version)?;
                let caps = parse::parse_apicapabilities(&list, &res.name)?;
                return Ok((ar, caps));
            }
        }
        Err(DiscoveryError::MissingKind(format!("{:?}", gvk)).into())
    }

    // shortcut method to give cheapest return for a pinned group
    pub(crate) async fn query_gv(client: &Client, gv: &GroupVersion) -> Result<Self> {
        let apiver = gv.api_version();
        let list = if gv.group.is_empty() {
            client.list_core_api_resources(&apiver).await?
        } else {
            client.list_api_group_resources(&apiver).await?
        };
        let data = GroupVersionData::new(gv.version.clone(), list)?;
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
    ///     let apigroup = discovery::group(&client, "apiregistration.k8s.io").await?;
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
    ///     let apigroup = discovery::group(&client, "apiregistration.k8s.io").await?;
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
