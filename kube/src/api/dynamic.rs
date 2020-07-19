use std::convert::TryFrom;
use crate::{
    api::{typed::Api, Resource},
    Client,
    Result, Error
};

use inflector::{cases::pascalcase::is_pascal_case, string::pluralize::to_plural};

use std::marker::PhantomData;

/// A dynamic builder for Resource
///
/// ```
/// use kube::api::Resource;
/// struct FooSpec {};
/// struct FooStatus {};
/// struct Foo {
///     spec: FooSpec,
///     status: FooStatus
/// };
/// let foos = Resource::dynamic("Foo") // <.spec.kind>
///    .group("clux.dev") // <.spec.group>
///    .version("v1")
///    .into_resource();
/// ```
#[derive(Default)]
pub struct DynamicResource {
    pub(crate) kind: String,
    pub(crate) version: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) namespace: Option<String>,
}

impl DynamicResource {
    /// Create a DynamicResource specifying the kind
    ///
    /// The kind must not be plural and it must be in PascalCase
    /// **Note:** You **must** call `group` and `version` to successfully convert
    /// this object into something useful
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

    /// Consume the DynamicResource and build a Resource
    ///
    /// Note this crashes on invalid group/version/kinds.
    /// Use `try_into_resource` to handle the errors.
    pub fn into_resource(self) -> Resource {
        Resource::try_from(self).unwrap()
    }

    /// Consume the DynamicResource and convert to an Api object
    ///
    /// Note this crashes on invalid group/version/kinds.
    /// Use `try_into_api` to handle the errors.
    pub fn into_api<K>(self, client: Client) -> Api<K> {
        let resource = Resource::try_from(self).unwrap();
        Api {
            client, resource,
            phantom: PhantomData,
        }
    }

    /// Consume the `DynamicResource` and attempt to build a `Resource`
    ///
    /// Equivalent to importing TryFrom trait into scope.
    pub fn try_into_resource(self) -> Result<Resource> {
        Resource::try_from(self)
    }

    /// Consume the `DynamicResource` and and attempt to convert to an Api object
    pub fn try_into_api<K>(self, client: Client) -> Result<Api<K>> {
        let resource = Resource::try_from(self)?;
        Ok(Api {
            client, resource,
            phantom: PhantomData,
        })
    }
}

impl TryFrom<DynamicResource> for Resource {
    type Error = crate::Error;
    fn try_from(rb: DynamicResource) -> Result<Self> {
        if rb.version.is_none() {
            return Err(Error::DynamicResource("Resource must have a version".into()));
        }
        if rb.group.is_none() {
            return Err(Error::DynamicResource("Resource must have a group (can be empty string)".into()));
        }
        if to_plural(&rb.kind) == rb.kind {
            return Err(Error::DynamicResource(format!("DynamicResource kind '{}' must not be pluralized", rb.kind)));
        }
        if !is_pascal_case(&rb.kind) {
            return Err(Error::DynamicResource(format!("DynamicResource kind '{}' must be PascalCase", rb.kind)));
        }
        let version = rb.version.unwrap();
        let group = rb.group.unwrap();
        Ok(Self {
            api_version: if group == "" {
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
    use crate::api::{PatchParams, PostParams, Resource};
    use crate::Result;
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

    #[tokio::test]
    #[ignore] // circle has no kubeconfig
    async fn convenient_custom_resource() {
        use crate::{Api, Client};
        use serde::{Deserialize, Serialize};
        #[derive(Clone, Debug, kube_derive::CustomResource, Deserialize, Serialize)]
        #[kube(group = "clux.dev", version = "v1", namespaced)]
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
