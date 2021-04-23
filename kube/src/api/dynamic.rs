use crate::api::{metadata::TypeMeta, GroupVersionKind, Resource};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{APIResource, ObjectMeta};
use std::borrow::Cow;

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
            kind,
            api_version,
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
        ApiResource::from_gvk_with_plural(
            gvk,
            &crate::api::metadata::to_plural(&gvk.kind.to_ascii_lowercase()),
        )
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

/// A dynamic representation of a kubernetes object
///
/// This will work with any non-list type object.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct DynamicObject {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Object metadata
    pub metadata: ObjectMeta,

    /// All other keys
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl DynamicObject {
    /// Create a DynamicObject with minimal values set from ApiResource.
    pub fn new(name: &str, resource: &ApiResource) -> Self {
        Self {
            types: Some(TypeMeta {
                api_version: resource.api_version.to_string(),
                kind: resource.kind.to_string(),
            }),
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                ..Default::default()
            },
            data: Default::default(),
        }
    }

    /// Attach dynamic data to a DynamicObject
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Attach a namespace to a DynamicObject
    pub fn within(mut self, ns: &str) -> Self {
        self.metadata.namespace = Some(ns.into());
        self
    }
}

impl Resource for DynamicObject {
    type DynamicType = ApiResource;

    fn group(dt: &ApiResource) -> Cow<'_, str> {
        dt.group.as_str().into()
    }

    fn version(dt: &ApiResource) -> Cow<'_, str> {
        dt.version.as_str().into()
    }

    fn kind(dt: &ApiResource) -> Cow<'_, str> {
        dt.kind.as_str().into()
    }

    fn api_version(dt: &ApiResource) -> Cow<'_, str> {
        dt.api_version.as_str().into()
    }

    fn plural(dt: &ApiResource) -> Cow<'_, str> {
        dt.plural.as_str().into()
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}

#[cfg(test)]
mod test {
    use crate::{
        api::{
            ApiResource, DynamicObject, GroupVersionKind, Patch, PatchParams, PostParams, Request, Resource,
        },
        Result,
    };
    #[test]
    fn raw_custom_resource() {
        let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
        let res = ApiResource::from_gvk(&gvk);
        let url = DynamicObject::url_path(&res, Some("myns"));

        let pp = PostParams::default();
        let req = Request::new(&url).create(&pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos?");
        let patch_params = PatchParams::default();
        let req = Request::new(url)
            .patch("baz", &patch_params, &Patch::Merge(()))
            .unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos/baz?");
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn raw_resource_in_default_group() -> Result<()> {
        let gvk = GroupVersionKind::gvk("", "v1", "Service");
        let api_resource = ApiResource::from_gvk(&gvk);
        let url = DynamicObject::url_path(&api_resource, None);
        let pp = PostParams::default();
        let req = Request::new(url).create(&pp, vec![])?;
        assert_eq!(req.uri(), "/api/v1/services?");
        Ok(())
    }

    #[cfg(feature = "derive")]
    #[tokio::test]
    #[ignore] // circle has no kubeconfig
    async fn convenient_custom_resource() {
        use crate as kube; // derive macro needs kube in scope
        use crate::{Api, Client, CustomResource};
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};
        #[derive(Clone, Debug, CustomResource, Deserialize, Serialize, JsonSchema)]
        #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
        struct FooSpec {
            foo: String,
        }
        let client = Client::try_default().await.unwrap();

        let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
        let api_resource = ApiResource::from_gvk(&gvk);
        let a1: Api<DynamicObject> = Api::namespaced_with(client.clone(), "myns", &api_resource);
        let a2: Api<Foo> = Api::namespaced(client.clone(), "myns");

        // make sure they return the same url_path through their impls
        assert_eq!(a1.request.url_path, a2.request.url_path);
    }
}
