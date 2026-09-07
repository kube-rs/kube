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
use kube_core::{DynamicResourceScope, NamespaceResourceScope, discovery::Scope};
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
    /// Whether requests should use metadata-only Accept headers
    /// (cached from `K::metadata_api()` at construction so that `impl<K> Api<K>`
    /// method blocks don't have to tighten to `K: Resource`).
    pub(crate) metadata_api: bool,
    /// Note: Using `iter::Empty` over `PhantomData`, because we never actually keep any
    /// `K` objects, so `Empty` better models our constraints (in particular, `Empty<K>`
    /// is `Send`, even if `K` may not be).
    pub(crate) _phantom: std::iter::Empty<K>,
}

/// Which namespaces an [`Api`] built from discovery should cover
///
/// Passed to [`Api::scoped_with`] and [`discovery::pinned_api`](crate::discovery::pinned_api),
/// where it only takes effect for namespaced kinds. A cluster scoped kind has no namespace to
/// pick, so the selection is ignored there rather than building a url the apiserver does not
/// serve.
///
/// `From` conversions are provided for the shapes a namespace usually arrives in, and an
/// absent namespace maps to [`Namespaces::Default`]:
///
/// ```
/// # use kube::api::Namespaces;
/// # let object_namespace: Option<&str> = None;
/// assert_eq!(Namespaces::from("ns1"), Namespaces::One("ns1"));
/// assert_eq!(Namespaces::from(object_namespace), Namespaces::Default);
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Namespaces<'a> {
    /// Every namespace
    ///
    /// For a namespaced kind this can only `list` and `watch`, as with [`Api::all_with`].
    All,
    /// The client's default namespace, as with [`Api::default_namespaced_with`]
    Default,
    /// One specific namespace, as with [`Api::namespaced_with`]
    ///
    /// [`Api::scoped_with`] treats an empty name the same as [`Namespaces::Default`], since an
    /// empty `metadata.namespace` is the other spelling of "no namespace given".
    One(&'a str),
}

impl<'a> From<&'a str> for Namespaces<'a> {
    fn from(ns: &'a str) -> Self {
        Self::One(ns)
    }
}

impl<'a> From<Option<&'a str>> for Namespaces<'a> {
    /// An object without a namespace is not necessarily cluster scoped, so this maps to
    /// [`Namespaces::Default`] rather than [`Namespaces::All`], which could only list and watch.
    fn from(ns: Option<&'a str>) -> Self {
        ns.map_or(Self::Default, Self::One)
    }
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
            metadata_api: K::metadata_api(),
            _phantom: std::iter::empty(),
        }
    }

    /// Resource in the namespaces selected, honouring the discovered [`Scope`]
    ///
    /// Discovery hands back an [`ApiResource`] and its [`ApiCapabilities`] together, and the
    /// capabilities carry the [`Scope`]. Pass that scope in and this constructor applies the
    /// selection only where it is meaningful, so callers do not have to branch on it themselves:
    ///
    /// ```no_run
    /// # use kube::{Api, Client, api::{DynamicObject, Namespaces}, discovery::Discovery};
    /// # async fn wrapper() -> Result<(), Box<dyn std::error::Error>> {
    /// # let client: Client = todo!();
    /// # let object: DynamicObject = todo!();
    /// let discovery = Discovery::new(client.clone()).run().await?;
    /// if let Some((ar, caps)) = discovery.resolve_object(&object) {
    ///     let ns = Namespaces::from(object.metadata.namespace.as_deref());
    ///     let api: Api<DynamicObject> = Api::scoped_with(client, ns, &ar, &caps.scope);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// A [`Namespaces`] selection is only meaningful for a namespaced kind; for a cluster scoped
    /// one it is ignored and the cluster wide url is used. See [`Namespaces`] for what each
    /// selection maps to.
    ///
    /// Only the [`Scope`] is needed, so a scope known without discovery works too, e.g. one read
    /// off a [`CustomResourceDefinition`]'s `spec.scope`.
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    ///
    /// # Warning
    ///
    /// [`Namespaces::All`] on a namespaced kind **can only `list` and `watch`**, as with
    /// [`Api::all_with`]. Other verbs need a specific namespace.
    ///
    /// [`Scope`]: crate::discovery::Scope
    /// [`ApiResource`]: crate::discovery::ApiResource
    /// [`ApiCapabilities`]: crate::discovery::ApiCapabilities
    /// [`CustomResourceDefinition`]: k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition
    pub fn scoped_with(client: Client, ns: Namespaces<'_>, dyntype: &K::DynamicType, scope: &Scope) -> Self
    where
        K: Resource<Scope = DynamicResourceScope>,
    {
        match (scope, ns) {
            (Scope::Cluster, _) => Self::all_with(client, dyntype),
            (Scope::Namespaced, Namespaces::All) => Self::all_with(client, dyntype),
            // an empty name is the other spelling of "no namespace given", so it lands where an
            // absent one does
            (Scope::Namespaced, Namespaces::Default | Namespaces::One("")) => {
                Self::default_namespaced_with(client, dyntype)
            }
            (Scope::Namespaced, Namespaces::One(ns)) => Self::namespaced_with(client, ns, dyntype),
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
            metadata_api: _,
            _phantom,
        } = self;
        f.debug_struct("Api")
            .field("request", &request)
            .field("client", &"...")
            .field("namespace", &namespace)
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

    // The full (scope x selection) matrix, since the scope is what decides whether the
    // selection applies at all.
    #[tokio::test]
    async fn scoped_with_lets_the_discovered_scope_decide() {
        use crate::api::{ApiResource, DynamicObject, Namespaces};
        use kube_core::discovery::Scope;

        let (mock_service, _handle) = mock::pair::<Request<Body>, Response<Body>>();
        // deliberately not "default", so `Namespaces::Default` is distinguishable from `One`
        let client = Client::new(mock_service, "kube-rs-test");
        let cm = ApiResource::erase::<corev1::ConfigMap>(&());
        let node = ApiResource::erase::<corev1::Node>(&());
        let build = |ns, ar, scope| {
            let api: Api<DynamicObject> = Api::scoped_with(client.clone(), ns, ar, scope);
            (api.namespace().map(String::from), api.resource_url().to_string())
        };

        // namespaced kind: the selection decides
        assert_eq!(build(Namespaces::All, &cm, &Scope::Namespaced), (
            None,
            "/api/v1/configmaps".into()
        ));
        assert_eq!(build(Namespaces::Default, &cm, &Scope::Namespaced), (
            Some("kube-rs-test".into()),
            "/api/v1/namespaces/kube-rs-test/configmaps".into()
        ));
        assert_eq!(build(Namespaces::One("ns1"), &cm, &Scope::Namespaced), (
            Some("ns1".into()),
            "/api/v1/namespaces/ns1/configmaps".into()
        ));
        // an empty name is the other spelling of "no namespace given", so it lands with Default
        assert_eq!(build(Namespaces::One(""), &cm, &Scope::Namespaced), (
            Some("kube-rs-test".into()),
            "/api/v1/namespaces/kube-rs-test/configmaps".into()
        ));

        // cluster scoped kind: the selection is ignored, including an explicit namespace
        for ns in [Namespaces::All, Namespaces::Default, Namespaces::One("ns1")] {
            assert_eq!(build(ns, &node, &Scope::Cluster), (None, "/api/v1/nodes".into()));
        }
    }

    #[test]
    fn namespaces_conversions_default_when_absent() {
        use crate::api::Namespaces;

        assert_eq!(Namespaces::from("ns1"), Namespaces::One("ns1"));
        assert_eq!(Namespaces::from(Some("ns1")), Namespaces::One("ns1"));
        assert_eq!(Namespaces::from(None), Namespaces::Default);
    }
}
