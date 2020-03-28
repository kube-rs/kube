use crate::{
    api::{typed::Api, Resource},
    Client,
};
use inflector::{cases::pascalcase::is_pascal_case, string::pluralize::to_plural};
use std::marker::PhantomData;

/// A data equivalent of the Resource trait for for Custom Resources
///
/// This is the smallest amount of info we need to run the API against a CR
/// The version, and group must be set by the user.
///
/// Prefer using #[derive(CustomResource)] from `kube-derive` over this.
pub struct CustomResource {
    kind: String,
    group: String,
    version: String,
    api_version: String,
    namespace: Option<String>,
}

impl CustomResource {
    /// Construct a CrBuilder
    pub fn kind(kind: &str) -> CrBuilder {
        CrBuilder::kind(kind)
    }
}

/// A builder for CustomResource
#[derive(Default)]
pub struct CrBuilder {
    pub(crate) kind: String,
    pub(crate) version: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) namespace: Option<String>,
}
impl CrBuilder {
    /// Create a CrBuilder
    ///
    /// ```
    /// use kube::api::{CustomResource, Resource};
    /// struct FooSpec {};
    /// struct FooStatus {};
    /// struct Foo {
    ///     spec: FooSpec,
    ///     status: FooStatus
    /// };
    /// let foos : Resource = CustomResource::kind("Foo") // <.spec.kind>
    ///    .group("clux.dev") // <.spec.group>
    ///    .version("v1")
    ///    .into_resource();
    /// ```
    fn kind(kind: &str) -> Self {
        assert!(to_plural(kind) != kind); // no plural in kind
        assert!(is_pascal_case(&kind)); // PascalCase kind
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

    // Consume the CrBuilder and build a CustomResource
    pub fn build(self) -> CustomResource {
        let version = self.version.expect("Crd must have a version");
        let group = self.group.expect("Crd must have a group");
        CustomResource {
            api_version: format!("{}/{}", group, version),
            kind: self.kind,
            version,
            group,
            namespace: self.namespace,
        }
    }

    // Consume the CrBuilder and convert to an Api object
    pub fn into_api<K>(self, client: Client) -> Api<K> {
        let crd = self.build();
        Api {
            client,
            api: crd.into(),
            phantom: PhantomData,
        }
    }

    // Consume the CrBuilder and convert to a Resource object
    pub fn into_resource(self) -> Resource {
        let crd = self.build();
        crd.into()
    }
}

/// Make Resource useable on CRDs without k8s_openapi
impl From<CustomResource> for Resource {
    fn from(c: CustomResource) -> Self {
        Self {
            api_version: c.api_version,
            kind: c.kind,
            group: c.group,
            version: c.version,
            namespace: c.namespace,
        }
    }
}

/// Make Api useable on CRDs without k8s_openapi
impl CustomResource {
    pub fn into_api<K>(self, client: Client) -> Api<K> {
        Api {
            client,
            api: self.into(),
            phantom: PhantomData,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::api::{CustomResource, PatchParams, PostParams, Resource};
    #[test]
    fn raw_custom_resource() {
        let r: Resource = CustomResource::kind("Foo")
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

    #[tokio::test]
    #[ignore] // circle has no kube config
    async fn convenient_custom_resource() {
        use crate::{Api, Client, Configuration};
        #[derive(Clone, Debug, kube_derive::CustomResource, Deserialize, Serialize)]
        #[kube(group = "clux.dev", version = "v1", namespaced)]
        struct FooSpec {
            foo: String,
        };
        let client = Client::from(Configuration::infer().await.unwrap());
        let r1: Api<Foo> = Api::namespaced(client.clone(), "myns");
        impl crate::api::resource::NamespaceScopedResource for Foo {} // TODO: This in kube-derive

        let r2: Api<Foo> = CustomResource::kind("Foo")
            .group("clux.dev")
            .version("v1")
            .within("myns")
            .build()
            .into_api(client);
        assert_eq!(r1.api.api_version, r2.api.api_version);
        // ^ ensures that traits are implemented
    }
}
