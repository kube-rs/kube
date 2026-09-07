//! single use discovery utils
//!
//! These helpers provides a simpler discovery interface, but do not offer any built-in caching.
//!
//! This can provide specific information for 3 cases:
//! - single kind in a particular group at a pinned version via [`oneshot::pinned_kind`]
//! - all kinds in a group at pinned version: "apiregistration.k8s.io/v1" via [`oneshot::pinned_group`]
//! - all kinds/version combinations in a group: "apiregistration.k8s.io" via [`oneshot::group`]
//!
//! It can also skip straight to a ready to use [`Api`] via [`oneshot::pinned_api`].
//!
//! [`oneshot::group`]: crate::discovery::group
//! [`oneshot::pinned_group`]: crate::discovery::pinned_group
//! [`oneshot::pinned_kind`]: crate::discovery::pinned_kind
//! [`oneshot::pinned_api`]: crate::discovery::pinned_api

use super::ApiGroup;
use crate::{Api, Client, Error, Result, api::Namespaces, error::DiscoveryError};
use kube_core::{
    DynamicResourceScope, Resource, TypeMeta,
    discovery::{ApiCapabilities, ApiResource},
    gvk::{GroupVersion, GroupVersionKind, ParseGroupVersionError},
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
///         println!("Found APIService: {}", service.name_any());
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
///         println!("Found APIService: {}", service.name_any());
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
///         println!("Found APIService: {}", service.name_any());
///     }
///     Ok(())
/// }
/// ```
pub async fn pinned_kind(client: &Client, gvk: &GroupVersionKind) -> Result<(ApiResource, ApiCapabilities)> {
    ApiGroup::query_gvk(client, gvk).await
}

/// Discovers a single kind from a [`TypeMeta`] and returns a ready to use [`Api`]
///
/// A shortcut around [`oneshot::pinned_kind`](crate::discovery::pinned_kind) for when you have an
/// `apiVersion` + `kind` (e.g. off a [`DynamicObject`](crate::api::DynamicObject) or a manifest)
/// and want an [`Api`] rather than an [`ApiResource`]. A [`TypeMeta`] carries no namespace, so one
/// is selected separately, and the discovered [`Scope`](crate::discovery::Scope) decides whether
/// that selection applies.
///
/// ```no_run
/// use kube::{Client, api::{Api, DynamicObject, Namespaces, TypeMeta}, discovery, ResourceExt};
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let client = Client::try_default().await?;
///     let tm = TypeMeta { api_version: "apiregistration.k8s.io/v1".into(), kind: "APIService".into() };
///     let api: Api<DynamicObject> = discovery::pinned_api(&client, &tm, Namespaces::All).await?;
///     for service in api.list(&Default::default()).await? {
///         println!("Found APIService: {}", service.name_any());
///     }
///     Ok(())
/// }
/// ```
///
/// This costs one discovery request per call and discards the [`ApiCapabilities`] it looked up.
/// If you also need those (to check verbs via
/// [`ApiCapabilities::supports_operation`](crate::discovery::ApiCapabilities::supports_operation),
/// say), call [`oneshot::pinned_kind`](crate::discovery::pinned_kind) and hand the result to
/// [`Api::scoped_with`] yourself.
pub async fn pinned_api<K>(client: &Client, tm: &TypeMeta, ns: Namespaces<'_>) -> Result<Api<K>>
where
    K: Resource<DynamicType = ApiResource, Scope = DynamicResourceScope>,
{
    // NB: GroupVersion::from_str splits with splitn(2, '/'), so it never actually errors and the
    // map_err below is unreachable. An empty apiVersion does get through it as an empty version,
    // which would then query `/api/` and fail deserializing an APIVersions as an APIResourceList,
    // so reject that here rather than surfacing it as a serde error. `Discovery::resolve_typemeta`
    // takes an unparsed apiVersion too, but only looks in the cache and never issues that request.
    let gvk = GroupVersionKind::try_from(tm)
        .map_err(|ParseGroupVersionError(s)| Error::Discovery(DiscoveryError::InvalidGroupVersion(s)))?;
    if gvk.version.is_empty() {
        return Err(Error::Discovery(DiscoveryError::InvalidGroupVersion(format!(
            "{:?} has no version",
            tm.api_version
        ))));
    }
    let (ar, caps) = pinned_kind(client, &gvk).await?;
    Ok(Api::scoped_with(client.clone(), ns, &ar, &caps.scope))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::DynamicObject, client::Body};
    use http::{Request, Response};
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, APIResourceList};
    use tower_test::mock;

    // Serve one discovery response containing a namespaced and a cluster scoped kind, and
    // assert the url `pinned_api` ends up with. `group_version` drives which discovery url is
    // expected, since `query_gvk` dispatches core (`/api/v1`) separately from the rest.
    async fn pinned_api_for(
        group_version: &str,
        kind: &str,
        ns: Namespaces<'_>,
    ) -> Result<Api<DynamicObject>> {
        let (mock_service, mut handle) = mock::pair::<Request<Body>, Response<Body>>();
        // not "default", so `Namespaces::Default` is distinguishable from `One("default")`
        let client = Client::new(mock_service, "kube-rs-test");
        let resource = |name: &str, kind: &str, namespaced: bool| APIResource {
            name: name.to_string(),
            kind: kind.to_string(),
            namespaced,
            verbs: vec!["get".to_string(), "list".to_string()],
            ..Default::default()
        };
        let list = APIResourceList {
            group_version: group_version.to_string(),
            resources: vec![
                resource("configmaps", "ConfigMap", true),
                resource("nodes", "Node", false),
                resource("deployments", "Deployment", true),
            ],
        };
        let served = tokio::spawn(async move {
            let (request, send) = handle.next_request().await.expect("discovery is queried");
            let body = serde_json::to_vec(&list).unwrap();
            send.send_response(Response::builder().body(Body::from(body)).unwrap());
            request.uri().path().to_string()
        });
        let tm = TypeMeta {
            api_version: group_version.to_string(),
            kind: kind.to_string(),
        };
        let api = pinned_api(&client, &tm, ns).await;
        // Dropping the local handle closes the mock when `pinned_api` returned an error without
        // querying, so that case fails immediately. On the success path the returned `Api` holds
        // its own clone, so the timeout is what stops an unqueried mock pending forever.
        drop(client);
        // Asserted out here because a panic inside the responder never sends the response, which
        // would deadlock the client side rather than surfacing the mismatch.
        let expected_url = if group_version.contains('/') {
            format!("/apis/{group_version}")
        } else {
            format!("/api/{group_version}")
        };
        let served = tokio::time::timeout(std::time::Duration::from_secs(5), served)
            .await
            .expect("discovery is queried")
            .unwrap();
        assert_eq!(served, expected_url);
        api
    }

    #[tokio::test]
    async fn pinned_api_honours_the_selection_for_namespaced_kinds() {
        let api = pinned_api_for("apps/v1", "Deployment", Namespaces::One("ns1"))
            .await
            .unwrap();
        assert_eq!(api.resource_url(), "/apis/apps/v1/namespaces/ns1/deployments");
    }

    #[tokio::test]
    async fn pinned_api_ignores_the_selection_for_cluster_scoped_kinds() {
        for ns in [Namespaces::All, Namespaces::Default, Namespaces::One("ns1")] {
            let api = pinned_api_for("v1", "Node", ns).await.unwrap();
            assert_eq!(api.resource_url(), "/api/v1/nodes");
        }
    }

    // The core group takes the other arm of `query_gvk`'s dispatch, and core kinds
    // (Pod/ConfigMap/Secret) are the likeliest input to this helper.
    #[tokio::test]
    async fn pinned_api_resolves_core_group_kinds() {
        let api = pinned_api_for("v1", "ConfigMap", Namespaces::One("ns1"))
            .await
            .unwrap();
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns1/configmaps");

        let api = pinned_api_for("v1", "ConfigMap", Namespaces::Default).await.unwrap();
        assert_eq!(api.resource_url(), "/api/v1/namespaces/kube-rs-test/configmaps");
    }

    // An empty apiVersion would otherwise reach `/api/` and fail deserializing the APIVersions
    // response as an APIResourceList, which says nothing about what the caller got wrong.
    #[tokio::test]
    async fn pinned_api_rejects_an_empty_api_version() {
        let (mock_service, handle) = mock::pair::<Request<Body>, Response<Body>>();
        // dropped so an unguarded request fails the test rather than pending forever
        drop(handle);
        let client = Client::new(mock_service, "default");
        let tm = TypeMeta {
            api_version: String::new(),
            kind: "ConfigMap".to_string(),
        };
        let err = pinned_api::<DynamicObject>(&client, &tm, Namespaces::All)
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Discovery(DiscoveryError::InvalidGroupVersion(_))),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn pinned_api_reports_an_undiscovered_kind() {
        let err = pinned_api_for("apps/v1", "DoesNotExist", Namespaces::All)
            .await
            .unwrap_err();
        assert!(
            matches!(err, Error::Discovery(DiscoveryError::MissingKind(_))),
            "unexpected error: {err:?}"
        );
    }
}
