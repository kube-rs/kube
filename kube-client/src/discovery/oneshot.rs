//! single use discovery utils
//!
//! These helpers provides a simpler discovery interface, but do not offer any built-in caching.
//!
//! This can provide specific information for 3 cases:
//! - single kind in a particular group at a pinned version via [`oneshot::pinned_kind`]
//! - all kinds in a group at pinned version: "apiregistration.k8s.io/v1" via [`oneshot::pinned_group`]
//! - all kinds/version combinations in a group: "apiregistration.k8s.io" via [`oneshot::group`]
//!
//! [`oneshot::group`]: crate::discovery::group
//! [`oneshot::pinned_group`]: crate::discovery::pinned_group
//! [`oneshot::pinned_kind`]: crate::discovery::pinned_kind

use super::ApiGroup;
use crate::{error::DiscoveryError, Client, Error, Result};
use kube_core::{
    discovery::{ApiCapabilities, ApiResource},
    gvk::{GroupVersion, GroupVersionKind},
};

/// Discovers all APIs available under a certain group at all versions
///
/// This is recommended if you work with one group, but do not want to pin the version
/// of the apigroup. You can instead work with a recommended version (preferred or latest).
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
pub async fn group(client: &Client, apigroup: &str) -> Result<ApiGroup> {
    if apigroup == ApiGroup::CORE_GROUP {
        let coreapis = client.list_core_api_versions().await?;
        return ApiGroup::query_core(client, coreapis).await;
    } else {
        let api_groups = client.list_api_groups().await?;
        for g in api_groups.groups {
            if g.name != apigroup {
                continue;
            }
            return ApiGroup::query_apis(client, g).await;
        }
    }
    Err(Error::Discovery(DiscoveryError::MissingApiGroup(
        apigroup.to_string(),
    )))
}

/// Discovers all APIs available under a certain group at a pinned version
///
/// This is a cheaper variant of [`oneshot::group`](crate::discovery::oneshot::group) when you know what version you want.
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::try_default().await?;
///     let gv = "apiregistration.k8s.io/v1".parse()?;
///     let apigroup = discovery::pinned_group(&client, &gv).await?;
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
/// If you only need a single `kind`, [`oneshot::pinned_kind`](crate::discovery::pinned_kind) is the best solution.
pub async fn pinned_group(client: &Client, gv: &GroupVersion) -> Result<ApiGroup> {
    ApiGroup::query_gv(client, gv).await
}

/// Single discovery for a single GVK
///
/// This is an optimized function that avoids the unnecessary listing of api groups.
/// It merely requests the api group resources for the specified apigroup, and then resolves the kind.
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject, GroupVersionKind}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::try_default().await?;
///     let gvk = GroupVersionKind::gvk("apiregistration.k8s.io", "v1", "APIService");
///     let (ar, caps) = discovery::pinned_kind(&client, &gvk).await?;
///     let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
///     for service in api.list(&Default::default()).await? {
///         println!("Found APIService: {}", service.name());
///     }
///     Ok(())
/// }
/// ```
pub async fn pinned_kind(client: &Client, gvk: &GroupVersionKind) -> Result<(ApiResource, ApiCapabilities)> {
    ApiGroup::query_gvk(client, gvk).await
}
