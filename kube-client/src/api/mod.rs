//! API helpers for structured interaction with the Kubernetes API

mod core_methods;
#[cfg(feature = "ws")] mod remote_command;
use std::fmt::Debug;

#[cfg(feature = "ws")] pub use remote_command::{AttachedProcess, TerminalSize};
#[cfg(feature = "ws")] mod portforward;
#[cfg(feature = "ws")] pub use portforward::Portforwarder;

mod subresource;
#[cfg_attr(docsrs, doc(cfg(feature = "k8s_if_ge_1_33")))]
pub use subresource::Resize;
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub use subresource::{Attach, AttachParams, Ephemeral, Execute, Portforward};
pub use subresource::{Evict, EvictParams, Log, LogParams, ScaleSpec, ScaleStatus};

mod util;

pub mod entry;

// Re-exports from kube-core
#[cfg(feature = "admission")]
#[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
pub use kube_core::admission;
pub(crate) use kube_core::params;
use kube_core::{DynamicResourceScope, NamespaceResourceScope, NamespaceScope, discovery::Scope};
pub use kube_core::{
    Resource, ResourceExt,
    dynamic::{ApiResource, DynamicObject},
    gvk::{GroupVersionKind, GroupVersionResource},
    metadata::{ListMeta, ObjectMeta, PartialObjectMeta, PartialObjectMetaExt, TypeMeta},
    object::{NotUsed, Object, ObjectList},
    request::Request,
    watch::WatchEvent,
};
pub use params::{
    DeleteParams, GetParams, ListParams, Patch, PatchParams, PostParams, Preconditions, PropagationPolicy,
    ValidationDirective, VersionMatch, WatchParams,
};

use crate::Client;
/// The generic Api abstraction
///
/// This abstracts over a [`Request`] and a type `K` so that
/// we get automatic serialization/deserialization on the api calls
/// implemented by the dynamic [`Resource`].
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
#[derive(Clone)]
pub struct Api<K> {
    /// The request builder object with its resource dependent url
    pub(crate) request: Request,
    /// The client to use (from this library)
    pub(crate) client: Client,
    namespace: Option<String>,
    /// The type-erased resource, kept so that [`Api::constrain`] can rebuild the url through
    /// [`Resource::url_path`] rather than editing the finished one
    ///
    /// Erased for the same reason `metadata_api` is a bool: a non-generic field needs no bound on
    /// the struct.
    resource: ApiResource,
    /// The [`Scope`] of the kind, when it is known
    ///
    /// Only [`Api::dynamic`] is told one; the other constructors leave it `None`, which makes
    /// [`Api::constrain`] apply a namespace unconditionally.
    scope: Option<Scope>,
    /// Whether requests should use metadata-only Accept headers
    /// (cached from `K::metadata_api()` at construction so that `impl<K> Api<K>`
    /// method blocks don't have to tighten to `K: Resource`).
    pub(crate) metadata_api: bool,
    /// Note: Using `iter::Empty` over `PhantomData`, because we never actually keep any
    /// `K` objects, so `Empty` better models our constraints (in particular, `Empty<K>`
    /// is `Send`, even if `K` may not be).
    pub(crate) _phantom: std::iter::Empty<K>,
}

/// Api constructors for Resource implementors with custom DynamicTypes
///
/// This generally means resources created via [`DynamicObject`](crate::api::DynamicObject).
impl<K: Resource> Api<K> {
    /// Return a reference to the namespace of this [`Api`] instance, if any
    pub fn namespace(&self) -> Option<&str> {
        self.namespace.as_deref()
    }

    /// Cluster level resources, or resources viewed across all namespaces
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    ///
    /// # Warning
    ///
    /// This variant **can only `list` and `watch` namespaced resources** and is commonly used with a `watcher`.
    /// If you need to create/patch/replace/get on a namespaced resource, you need a separate `Api::namespaced`.
    pub fn all_with(client: Client, dyntype: &K::DynamicType) -> Self {
        let url = K::url_path(dyntype, None);
        Self {
            client,
            request: Request::new(url),
            namespace: None,
            resource: ApiResource::erase::<K>(dyntype),
            scope: None,
            metadata_api: K::metadata_api(),
            _phantom: std::iter::empty(),
        }
    }

    /// Namespaced resource within a given namespace
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    pub fn namespaced_with(client: Client, ns: &str, dyntype: &K::DynamicType) -> Self
    where
        K: Resource<Scope = DynamicResourceScope>,
    {
        // TODO: inspect dyntype scope to verify somehow?
        let url = K::url_path(dyntype, Some(ns));
        Self {
            client,
            request: Request::new(url),
            namespace: Some(ns.to_string()),
            resource: ApiResource::erase::<K>(dyntype),
            scope: None,
            metadata_api: K::metadata_api(),
            _phantom: std::iter::empty(),
        }
    }

    /// Namespaced resource within the default namespace
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    ///
    /// The namespace is either configured on `context` in the kubeconfig
    /// or falls back to `default` when running locally, and it's using the service account's
    /// namespace when deployed in-cluster.
    pub fn default_namespaced_with(client: Client, dyntype: &K::DynamicType) -> Self
    where
        K: Resource<Scope = DynamicResourceScope>,
    {
        let ns = client.default_namespace().to_string();
        Self::namespaced_with(client, &ns, dyntype)
    }

    /// Resource of a kind only known at runtime, remembering its [`Scope`]
    ///
    /// The url starts out cluster wide, exactly as [`all_with`](Api::all_with) builds it, and the
    /// [`Scope`] is kept so that [`constrain`](Api::constrain) can apply a namespace the way
    /// `kubectl -n` does: to a namespaced kind, but not to a cluster scoped one.
    ///
    /// ```no_run
    /// # use kube::{Api, Client, api::{ApiResource, DynamicObject}, discovery::Scope};
    /// # use k8s_openapi::api::core::v1::Pod;
    /// # let client: Client = todo!();
    /// let ar = ApiResource::erase::<Pod>(&());
    /// let api: Api<DynamicObject> = Api::dynamic(client, &ar, &Scope::Namespaced);
    /// assert_eq!(api.resource_url(), "/api/v1/pods");
    /// ```
    ///
    /// The [`Scope`] usually arrives from discovery, on the [`ApiCapabilities`] handed back next
    /// to the [`ApiResource`], but any source works, e.g. a [`CustomResourceDefinition`]'s
    /// `spec.scope`. It has to be passed in because it cannot be read off the dyntype, which is
    /// what the `TODO` on [`namespaced_with`](Api::namespaced_with) was asking for.
    ///
    /// # Warning
    ///
    /// A cluster wide url **can only `list` and `watch`** a namespaced kind, as with
    /// [`Api::all_with`]. The other verbs need a [`constrain`](Api::constrain) first.
    ///
    /// [`Scope`]: crate::discovery::Scope
    /// [`ApiResource`]: crate::discovery::ApiResource
    /// [`ApiCapabilities`]: crate::discovery::ApiCapabilities
    /// [`CustomResourceDefinition`]: k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition
    pub fn dynamic(client: Client, dyntype: &K::DynamicType, scope: &Scope) -> Self
    where
        K: Resource<DynamicType = ApiResource, Scope = DynamicResourceScope>,
    {
        Self {
            scope: Some(scope.clone()),
            ..Self::all_with(client, dyntype)
        }
    }

    /// Constrain this [`Api`] to a namespace, the way `kubectl -n` does
    ///
    /// Rebuilds the collection url with `namespaces/<ns>/` when the kind is namespaced, and does
    /// nothing when it is cluster scoped, mirroring `kubectl get nodes -n whatever`, which drops
    /// the flag rather than erroring. Constraining again replaces the namespace rather than
    /// nesting it, so this is also how to move a typed [`Api`] between namespaces without
    /// building a second one:
    ///
    /// ```no_run
    /// # use kube::{Api, Client};
    /// # use k8s_openapi::api::core::v1::Pod;
    /// # let client: Client = todo!();
    /// let pods: Api<Pod> = Api::namespaced(client, "ns1").constrain("ns2");
    /// assert_eq!(pods.resource_url(), "/api/v1/namespaces/ns2/pods");
    /// ```
    ///
    /// A cluster scoped kind has no namespace to name, so it is rejected at compile time:
    ///
    /// ```compile_fail
    /// # use kube::{Api, Client};
    /// # use k8s_openapi::api::core::v1::Node;
    /// # let client: Client = todo!();
    /// let nodes: Api<Node> = Api::all(client).constrain("ns1"); // resource not namespaced!
    /// ```
    ///
    /// ```no_run
    /// # use kube::{Api, Client, api::{ApiResource, DynamicObject}, discovery::Scope};
    /// # use k8s_openapi::api::core::v1::{Node, Pod};
    /// # let client: Client = todo!();
    /// let pods = Api::<DynamicObject>::dynamic(client.clone(), &ApiResource::erase::<Pod>(&()), &Scope::Namespaced);
    /// assert_eq!(pods.constrain("kube-system").resource_url(), "/api/v1/namespaces/kube-system/pods");
    ///
    /// let nodes = Api::<DynamicObject>::dynamic(client, &ApiResource::erase::<Node>(&()), &Scope::Cluster);
    /// assert_eq!(nodes.constrain("kube-system").resource_url(), "/api/v1/nodes");
    /// ```
    ///
    /// The url is rebuilt through [`Resource::url_path`] from the type-erased resource the
    /// [`Api`] keeps, so this never edits the previous url and there is no second copy of the
    /// path format to keep in sync.
    ///
    /// An empty `ns` is how Kubernetes spells "every namespace", and it lands there by the same
    /// route, as the `None` [`Resource::url_path`] takes: the constraint is dropped and the
    /// cluster wide url [`Api::dynamic`] starts with comes back, which also makes it the inverse
    /// of a `constrain`. A kubeconfig context with `namespace: ""` therefore widens
    /// [`constrain_default`](Api::constrain_default) rather than building `namespaces//`.
    ///
    /// The runtime [`Scope`] check only matters for a dynamic kind, and is only known on an
    /// [`Api`] from [`Api::dynamic`]. A typed kind is statically namespaced to be here at all, so
    /// there is nothing to consult. A dynamic [`Api`] from one of the other constructors records
    /// no scope, and there the namespace is applied unconditionally, as
    /// [`namespaced_with`](Api::namespaced_with) would; a cluster scoped kind then builds a url
    /// the apiserver rejects, rather than quietly dropping the namespace that was asked for.
    ///
    /// [`Scope`]: crate::discovery::Scope
    #[must_use]
    pub fn constrain(mut self, ns: &str) -> Self
    where
        K::Scope: NamespaceScope,
    {
        if self.scope == Some(Scope::Cluster) {
            return self;
        }
        self.namespace = (!ns.is_empty()).then(|| ns.to_string());
        // the stored resource is the erased dyntype, so this is `Resource::url_path` on its
        // canonical carrier - the same string `K::url_path` would build
        self.request = Request::new(DynamicObject::url_path(&self.resource, self.namespace.as_deref()));
        self
    }

    /// Constrain this [`Api`] to the [`Client`]'s default namespace
    ///
    /// The kubectl equivalent of passing no `-n` at all. A shorthand for
    /// `api.constrain(client.default_namespace())` that does not need the [`Client`] to still be
    /// in scope, since the [`Api`] already holds one.
    ///
    /// ```no_run
    /// # use kube::{Api, Client, api::{ApiResource, DynamicObject}, discovery::Scope};
    /// # use k8s_openapi::api::core::v1::Pod;
    /// # let client: Client = todo!();
    /// let ar = ApiResource::erase::<Pod>(&());
    /// let api: Api<DynamicObject> = Api::dynamic(client, &ar, &Scope::Namespaced).constrain_default();
    /// ```
    #[must_use]
    pub fn constrain_default(self) -> Self
    where
        K::Scope: NamespaceScope,
    {
        let ns = self.client.default_namespace().to_string();
        self.constrain(&ns)
    }

    /// Consume self and return the [`Client`]
    pub fn into_client(self) -> Client {
        self.into()
    }

    /// Return a reference to the current resource url path
    pub fn resource_url(&self) -> &str {
        &self.request.url_path
    }
}

/// Api constructors for Resource implementors with Default DynamicTypes
///
/// This generally means structs implementing `k8s_openapi::Resource`.
impl<K: Resource> Api<K>
where
    <K as Resource>::DynamicType: Default,
{
    /// Cluster level resources, or resources viewed across all namespaces
    ///
    /// Namespace scoped resource allowing querying across all namespaces:
    ///
    /// ```no_run
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Pod;
    /// let api: Api<Pod> = Api::all(client);
    /// ```
    ///
    /// Cluster scoped resources also use this entrypoint:
    ///
    /// ```no_run
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Node;
    /// let api: Api<Node> = Api::all(client);
    /// ```
    ///
    /// # Warning
    ///
    /// This variant **can only `list` and `watch` namespaced resources** and is commonly used with a `watcher`.
    /// If you need to create/patch/replace/get on a namespaced resource, you need a separate `Api::namespaced`.
    pub fn all(client: Client) -> Self {
        Self::all_with(client, &K::DynamicType::default())
    }

    /// Namespaced resource within a given namespace
    ///
    /// ```no_run
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Pod;
    /// let api: Api<Pod> = Api::namespaced(client, "default");
    /// ```
    ///
    /// This will ONLY work on namespaced resources as set by `Scope`:
    ///
    /// ```compile_fail
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Node;
    /// let api: Api<Node> = Api::namespaced(client, "default"); // resource not namespaced!
    /// ```
    ///
    /// For dynamic type information, use [`Api::namespaced_with`] variants.
    pub fn namespaced(client: Client, ns: &str) -> Self
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        let dyntype = K::DynamicType::default();
        let url = K::url_path(&dyntype, Some(ns));
        Self {
            client,
            request: Request::new(url),
            namespace: Some(ns.to_string()),
            resource: ApiResource::erase::<K>(&dyntype),
            scope: None,
            metadata_api: K::metadata_api(),
            _phantom: std::iter::empty(),
        }
    }

    /// Namespaced resource within the default namespace
    ///
    /// The namespace is either configured on `context` in the kubeconfig
    /// or falls back to `default` when running locally, and it's using the service account's
    /// namespace when deployed in-cluster.
    ///
    /// ```no_run
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Pod;
    /// let api: Api<Pod> = Api::default_namespaced(client);
    /// ```
    ///
    /// This will ONLY work on namespaced resources as set by `Scope`:
    ///
    /// ```compile_fail
    /// # use kube::{Api, Client};
    /// # let client: Client = todo!();
    /// use k8s_openapi::api::core::v1::Node;
    /// let api: Api<Node> = Api::default_namespaced(client); // resource not namespaced!
    /// ```
    pub fn default_namespaced(client: Client) -> Self
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        let ns = client.default_namespace().to_string();
        Self::namespaced(client, &ns)
    }
}

impl<K> From<Api<K>> for Client {
    fn from(api: Api<K>) -> Self {
        api.client
    }
}

impl<K> Debug for Api<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Intentionally destructuring, to cause compile errors when new fields are added
        let Self {
            request,
            client: _,
            namespace,
            resource: _,
            scope,
            metadata_api: _,
            _phantom,
        } = self;
        f.debug_struct("Api")
            .field("request", &request)
            .field("client", &"...")
            .field("namespace", &namespace)
            .field("scope", &scope)
            .finish()
    }
}

/// Sanity test on scope restrictions
#[cfg(test)]
mod test {
    use crate::{Api, Client, client::Body};
    use k8s_openapi::api::core::v1 as corev1;

    use http::{Request, Response};
    use tower_test::mock;

    #[tokio::test]
    async fn scopes_should_allow_correct_interface() {
        let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
        let client = Client::new(mock_service, "default");

        let _: Api<corev1::Node> = Api::all(client.clone());
        let _: Api<corev1::Pod> = Api::default_namespaced(client.clone());
        let _: Api<corev1::PersistentVolume> = Api::all(client.clone());
        let _: Api<corev1::ConfigMap> = Api::namespaced(client, "default");
    }

    // deliberately not "default", so the client default is distinguishable from a literal
    fn mock_client() -> Client {
        let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
        Client::new(mock_service, "kube-rs-test")
    }

    // The (scope x constraint) matrix, since the scope is what decides whether `constrain`
    // applies at all.
    #[tokio::test]
    async fn constrain_applies_only_to_namespaced_kinds() {
        use crate::api::{ApiResource, DynamicObject};
        use k8s_openapi::api::apps::v1 as appsv1;
        use kube_core::discovery::Scope;

        let client = mock_client();
        let dynamic = |ar, scope| Api::<DynamicObject>::dynamic(client.clone(), ar, scope);
        let cm = ApiResource::erase::<corev1::ConfigMap>(&());
        let deploy = ApiResource::erase::<appsv1::Deployment>(&());
        let node = ApiResource::erase::<corev1::Node>(&());

        // a namespaced kind starts cluster wide, and `constrain` narrows it
        let api = dynamic(&cm, &Scope::Namespaced);
        assert_eq!(api.namespace(), None);
        assert_eq!(api.resource_url(), "/api/v1/configmaps");
        let api = api.constrain("ns1");
        assert_eq!(api.namespace(), Some("ns1"));
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns1/configmaps");

        // constraining again replaces the namespace rather than nesting it
        let api = api.constrain("ns2");
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns2/configmaps");

        // an empty namespace is "every namespace", so it undoes the constraint
        let api = api.constrain("");
        assert_eq!(api.namespace(), None);
        assert_eq!(api.resource_url(), "/api/v1/configmaps");

        // a non-core group has a longer prefix to splice into
        assert_eq!(
            dynamic(&deploy, &Scope::Namespaced).constrain("ns1").resource_url(),
            "/apis/apps/v1/namespaces/ns1/deployments"
        );

        // `constrain_default` picks the namespace off the client the Api already holds
        let api = dynamic(&cm, &Scope::Namespaced).constrain_default();
        assert_eq!(api.namespace(), Some("kube-rs-test"));
        assert_eq!(api.resource_url(), "/api/v1/namespaces/kube-rs-test/configmaps");

        // a cluster scoped kind drops the constraint, as `kubectl get nodes -n whatever` does
        for api in [
            dynamic(&node, &Scope::Cluster),
            dynamic(&node, &Scope::Cluster).constrain("ns1"),
            dynamic(&node, &Scope::Cluster).constrain_default(),
        ] {
            assert_eq!(api.namespace(), None);
            assert_eq!(api.resource_url(), "/api/v1/nodes");
        }
    }

    // A typed namespaced kind is statically namespaced, so `constrain` is just a namespace
    // switch on it, and a cluster scoped one does not get the method at all (doc `compile_fail`).
    #[tokio::test]
    async fn constrain_switches_namespaces_on_typed_apis() {
        let client = mock_client();

        let api: Api<corev1::ConfigMap> = Api::namespaced(client.clone(), "ns1").constrain("ns2");
        assert_eq!(api.namespace(), Some("ns2"));
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns2/configmaps");

        // narrowing an `all` Api is the same move in the other direction
        let api: Api<corev1::Pod> = Api::all(client.clone()).constrain("ns1");
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns1/pods");

        let api: Api<corev1::Pod> = Api::all(client).constrain_default();
        assert_eq!(api.resource_url(), "/api/v1/namespaces/kube-rs-test/pods");
    }

    // Without a scope there is nothing to consult, so the namespace is applied rather than
    // silently dropped - which also means it can rewrite what `namespaced_with` built.
    #[tokio::test]
    async fn constrain_applies_unconditionally_without_a_known_scope() {
        use crate::api::{ApiResource, DynamicObject};

        let client = mock_client();
        let cm = ApiResource::erase::<corev1::ConfigMap>(&());

        let api: Api<DynamicObject> = Api::all_with(client.clone(), &cm).constrain("ns1");
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns1/configmaps");

        let api: Api<DynamicObject> = Api::namespaced_with(client, "ns1", &cm).constrain("ns2");
        assert_eq!(api.resource_url(), "/api/v1/namespaces/ns2/configmaps");
    }
}
