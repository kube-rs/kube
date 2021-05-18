use crate::{gvk::GroupVersionKind, resource::Resource};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResource;

/// Contains information about Kubernetes API resources
/// which is enough for working with it.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ApiResource {
    /// Resource group, empty for core group.
    pub group: String,
    /// group version
    pub version: String,
    /// apiVersion of the resource (v1 for core group,
    /// groupName/groupVersions for other).
    pub api_version: String,
    /// Singular PascalCase name of the resource
    pub kind: String,
    /// Plural name of the resource
    pub plural: String,
}

impl ApiResource {
    /// Creates ApiResource from `meta::v1::APIResource` instance.
    ///
    /// `APIResource` objects can be extracted from [`Client::list_api_group_resources`](crate::Client::list_api_group_resources).
    /// If it does not specify version and/or group, they will be taken from `group_version`
    /// (otherwise the second parameter is ignored).
    ///
    /// ### Example usage:
    /// ```
    /// use kube::api::{ApiResource, Api, DynamicObject};
    /// # async fn scope(client: kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    /// let apps = client.list_api_group_resources("apps/v1").await?;
    /// for ar in &apps.resources {
    ///     let resource = ApiResource::from_apiresource(ar, &apps.group_version);
    ///     dbg!(&resource);
    ///     let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), "default", &resource);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_apiresource(ar: &APIResource, group_version: &str) -> Self {
        let gvsplit = group_version.splitn(2, '/').collect::<Vec<_>>();
        let (default_group, default_version) = match *gvsplit.as_slice() {
            [g, v] => (g, v), // standard case
            [v] => ("", v),   // core v1 case
            _ => unreachable!(),
        };
        let group = ar.group.clone().unwrap_or_else(|| default_group.into());
        let version = ar.version.clone().unwrap_or_else(|| default_version.into());
        let kind = ar.kind.to_string();
        let api_version = if group.is_empty() {
            version.clone()
        } else {
            format!("{}/{}", group, version)
        };
        let plural = ar.name.clone();
        ApiResource {
            group,
            version,
            api_version,
            kind,
            plural,
        }
    }

    /// Creates ApiResource by type-erasing another Resource
    pub fn erase<K: Resource>(dt: &K::DynamicType) -> Self {
        ApiResource {
            group: K::group(dt).to_string(),
            version: K::version(dt).to_string(),
            api_version: K::api_version(dt).to_string(),
            kind: K::kind(dt).to_string(),
            plural: K::plural(dt).to_string(),
        }
    }

    /// Creates ApiResource from group, version and kind.
    /// # Warning
    /// This function has to **guess** resource plural name.
    /// While it makes it best to guess correctly, sometimes it can
    /// be wrong, and using returned ApiResource will lead to incorrect
    /// api requests.
    pub fn from_gvk(gvk: &GroupVersionKind) -> Self {
        ApiResource::from_gvk_with_plural(gvk, &crate::resource::to_plural(&gvk.kind.to_ascii_lowercase()))
    }

    /// Creates ApiResource from group, version, kind and plural name.
    pub fn from_gvk_with_plural(gvk: &GroupVersionKind, plural: &str) -> Self {
        let api_version = match gvk.group.as_str() {
            "" => gvk.version.clone(),
            _ => format!("{}/{}", gvk.group, gvk.version),
        };
        ApiResource {
            group: gvk.group.clone(),
            version: gvk.version.clone(),
            api_version,
            kind: gvk.kind.clone(),
            plural: plural.to_string(),
        }
    }
}
