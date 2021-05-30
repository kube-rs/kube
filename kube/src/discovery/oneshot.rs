//! single use discovery utils
//!
//! These helpers provides a simpler discovery interface, but do not offer any built-in caching.
//!
//! This can provide specific information for 3 cases:
//! - single group discovery: "apiregistration.k8s.io" via [`oneshot::group`]
//! - single group at pinned version: "apiregistration.k8s.io/v1" via [`oneshot::gv`]
//! - single kind in a particular group at a pinned version via [`oneshot::gvk`]
//!
//! [`oneshot::group`]: crate::discovery::oneshot::group
//! [`oneshot::gv`]: crate::discovery::oneshot::gv
//! [`oneshot::gvk`]: crate::discovery::oneshot::gvk

use super::ApiGroup;
use crate::{error::DiscoveryError, Client, Result};
use kube_core::{
    discovery::{ApiCapabilities, ApiResource},
    gvk::{GroupVersion, GroupVersionKind},
};

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
///     let apigroup = discovery::oneshot::group(&client, "apiregistration.k8s.io").await?;
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
        return ApiGroup::query_core(&client, coreapis).await;
    } else {
        let api_groups = client.list_api_groups().await?;
        for g in api_groups.groups {
            if g.name != apigroup {
                continue;
            }
            return ApiGroup::query_apis(&client, g).await;
        }
    }
    Err(DiscoveryError::MissingApiGroup(apigroup.to_string()).into())
}

/// Discovers all APIs available under a certain group at a particular version and return the singular ApiGroup
///
/// This is a cheaper variant of [`oneshot::group`](crate::discovery::oneshot::group) when you know what version you want.
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), kube::Error> {
///     let client = Client::try_default().await?;
///     let gv = "apiregistration.k8s.io/v1".parse()?;
///     let apigroup = discovery::oneshot::gv(&client, &gv).await?;
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
/// If you only need a single `kind`, [`oneshot::gvk`](crate::discovery::oneshot::gvk) is the best solution.
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
///     let (ar, caps) = discovery::oneshot::gvk(&client, &gvk).await?;
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
