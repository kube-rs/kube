use crate::{
    api::{typed::Api, Resource},
    Client, Error, Result,
};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResource;
use std::convert::TryFrom;

use inflector::{cases::pascalcase::is_pascal_case, string::pluralize::to_plural};

use std::iter;

/// A dynamic builder for Resource
///
/// Can be used to interact with a dynamic api resources.
/// Can be constructed either from [`DynamicResource::from_api_resource`], or directly.
///
/// ### Direct usage
/// ```
/// use kube::api::Resource;
/// let foos = Resource::dynamic("Foo") // <.spec.kind>
///    .group("clux.dev") // <.spec.group>
///    .version("v1")
///    .into_resource();
/// ```
///
/// It is recommended to use [`kube::CustomResource`] (from kube's `derive` feature)
/// for CRD cases where you own a struct rather than this.
///
/// **Note:** You will need to implement [`k8s_openapi`] traits yourself to use the typed [`Api`]
/// with a [`Resource`] built from a [`DynamicResource`] (and this is not always feasible).
///
/// [`kube::CustomResource`]: crate::CustomResource
#[derive(Default)]
pub struct DynamicResource {
    pub(crate) kind: String,
    pub(crate) version: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) namespace: Option<String>,
}

impl DynamicResource {
    /// Creates `DynamicResource` from an [`APIResource`].
    ///
    /// `APIResource` objects can be extracted from [`Client::list_api_group_resources`].
    /// If it does not specify version and/or group, they will be taken
    /// from `group_version`.
    ///
    /// ### Example usage:
    /// ```
    /// use kube::api::DynamicResource;
    /// # async fn scope(client: kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    /// let apps = client.list_api_group_resources("apps/v1").await?;
    /// for ar in &apps.resources {
    ///     let dr = DynamicResource::from_api_resource(ar, &apps.group_version);
    ///     let r = dr.within("kube-system").into_resource();
    ///     dbg!(r);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`API Resource`]: k8s_openapi::apimachinery::pkg::apis::meta::v1::APIResource
    pub fn from_api_resource(ar: &APIResource, group_version: &str) -> Self {
        let gvsplit = group_version.splitn(2, '/').collect::<Vec<_>>();
        let (default_group, default_version) = match *gvsplit.as_slice() {
            [g, v] => (g, v), // standard case
            [v] => ("", v),   // core v1 case
            _ => unreachable!(),
        };
        let version = ar.version.clone().unwrap_or_else(|| default_version.into());
        let group = ar.group.clone().unwrap_or_else(|| default_group.into());
        DynamicResource {
            kind: ar.kind.to_string(),
            version: Some(version),
            group: Some(group),
            namespace: None,
        }
    }

    /// Create a `DynamicResource` specifying the kind.
    ///
    /// The kind must not be plural and it must be in PascalCase
    /// **Note:** You **must** call [`group`] and [`version`] to successfully convert
    /// this object into something useful.
    ///
    /// [`group`]: Self::group
    /// [`version`]: Self::version
    pub fn new(kind: &str) -> Self {
        Self {
            kind: kind.into(),
            ..Default::default()
        }
    }

    /// Set the api group of a custom resource
    pub fn group(mut self, group: &str) -> Self {
        self.group = Some(group.to_string());
        self
    }

    /// Set the api version of a custom resource
    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set the namespace of a custom resource
    pub fn within(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    /// Consume the `DynamicResource` and build a `Resource`.
    ///
    /// Note this crashes on invalid group/version/kinds.
    /// Use [`try_into_resource`](Self::try_into_resource) to handle the errors.
    pub fn into_resource(self) -> Resource {
        Resource::try_from(self).unwrap()
    }

    /// Consume the `DynamicResource` and convert to an `Api` object.
    ///
    /// Note this crashes on invalid group/version/kinds.
    /// Use [`try_into_api`](Self::try_into_api) to handle the errors.
    pub fn into_api<K>(self, client: Client) -> Api<K> {
        let resource = Resource::try_from(self).unwrap();
        Api {
            client,
            resource,
            phantom: iter::empty(),
        }
    }

    /// Consume the `DynamicResource` and attempt to build a `Resource`.
    ///
    /// Equivalent to importing TryFrom trait into scope.
    pub fn try_into_resource(self) -> Result<Resource> {
        Resource::try_from(self)
    }

    /// Consume the `DynamicResource` and and attempt to convert to an `Api` object.
    pub fn try_into_api<K>(self, client: Client) -> Result<Api<K>> {
        let resource = Resource::try_from(self)?;
        Ok(Api {
            client,
            resource,
            phantom: iter::empty(),
        })
    }
}

impl TryFrom<DynamicResource> for Resource {
    type Error = crate::Error;

    fn try_from(rb: DynamicResource) -> Result<Self> {
        if rb.version.is_none() {
            return Err(Error::DynamicResource(format!(
                "DynamicResource '{}' must have a version",
                rb.kind
            )));
        }
        if rb.group.is_none() {
            return Err(Error::DynamicResource(format!(
                "DynamicResource '{}' must have a group (can be empty string)",
                rb.kind
            )));
        }
        let version = rb.version.unwrap();
        let group = rb.group.unwrap();

        // pedantic conventions we enforce internally in kube-derive
        // but are broken by a few native / common custom resources such as istio, or
        // kinds matching: CRI*, *Options, *Metrics, CSI*, ENI*, API*
        if to_plural(&rb.kind) == rb.kind || !is_pascal_case(&rb.kind) {
            debug!("DynamicResource '{}' should be singular + PascalCase", rb.kind);
        }
        Ok(Self {
            api_version: if group.is_empty() {
                version.clone()
            } else {
                format!("{}/{}", group, version)
            },
            kind: rb.kind,
            version,
            group,
            namespace: rb.namespace,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::{
        api::{PatchParams, PostParams, Resource},
        Result,
    };
    #[test]
    fn raw_custom_resource() {
        let r = Resource::dynamic("Foo")
            .group("clux.dev")
            .version("v1")
            .within("myns")
            .into_resource();

        let pp = PostParams::default();
        let req = r.create(&pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos?");
        let patch_params = PatchParams::default();
        let req = r.patch("baz", &patch_params, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos/baz?");
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn raw_resource_in_default_group() -> Result<()> {
        let r = Resource::dynamic("Service")
            .group("")
            .version("v1")
            .try_into_resource()?;
        let pp = PostParams::default();
        let req = r.create(&pp, vec![])?;
        assert_eq!(req.uri(), "/api/v1/services?");
        Ok(())
    }

    #[cfg(feature = "derive")]
    #[tokio::test]
    #[ignore] // circle has no kubeconfig
    async fn convenient_custom_resource() {
        use crate::{Api, Client, CustomResource};
        use schemars::JsonSchema;
        use serde::{Deserialize, Serialize};
        #[derive(Clone, Debug, CustomResource, Deserialize, Serialize, JsonSchema)]
        #[kube(group = "clux.dev", version = "v1", kind = "Foo", namespaced)]
        struct FooSpec {
            foo: String,
        };
        let client = Client::try_default().await.unwrap();
        let a1: Api<Foo> = Api::namespaced(client.clone(), "myns");

        let a2: Api<Foo> = Resource::dynamic("Foo")
            .group("clux.dev")
            .version("v1")
            .within("myns")
            .into_api(client);
        assert_eq!(a1.resource.api_version, a2.resource.api_version);
        // ^ ensures that traits are implemented
    }
}
