//! API helpers for structured interaction with the Kubernetes API


mod core_methods;
#[cfg(feature = "ws")] mod remote_command;
use std::fmt::Debug;

#[cfg(feature = "ws")] pub use remote_command::AttachedProcess;
#[cfg(feature = "ws")] mod portforward;
#[cfg(feature = "ws")] pub use portforward::Portforwarder;

mod subresource;
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub use subresource::{Attach, AttachParams, Execute, Portforward};
pub use subresource::{Evict, EvictParams, Log, LogParams, ScaleSpec, ScaleStatus};

mod util;

pub mod entry;

// Re-exports from kube-core
#[cfg(feature = "admission")]
#[cfg_attr(docsrs, doc(cfg(feature = "admission")))]
pub use kube_core::admission;
pub(crate) use kube_core::params;
pub use kube_core::{
    dynamic::{ApiResource, DynamicObject},
    gvk::{GroupVersionKind, GroupVersionResource},
    metadata::{ListMeta, ObjectMeta, TypeMeta},
    object::{NotUsed, Object, ObjectList},
    request::Request,
    watch::WatchEvent,
    Resource, ResourceExt,
};
pub use params::{
    DeleteParams, ListParams, Patch, PatchParams, PostParams, Preconditions, PropagationPolicy,
    ValidationDirective,
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
    /// Note: Using `iter::Empty` over `PhantomData`, because we never actually keep any
    /// `K` objects, so `Empty` better models our constraints (in particular, `Empty<K>`
    /// is `Send`, even if `K` may not be).
    pub(crate) _phantom: std::iter::Empty<K>,
}

/// Api constructors for Resource implementors with custom DynamicTypes
///
/// This generally means resources created via [`DynamicObject`](crate::api::DynamicObject).
impl<K: Resource> Api<K> {
    /// Cluster level resources, or resources viewed across all namespaces
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    pub fn all_with(client: Client, dyntype: &K::DynamicType) -> Self {
        let url = K::url_path(dyntype, None);
        Self {
            client,
            request: Request::new(url),
            namespace: None,
            _phantom: std::iter::empty(),
        }
    }

    /// Namespaced resource within a given namespace
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    pub fn namespaced_with(client: Client, ns: &str, dyntype: &K::DynamicType) -> Self {
        let url = K::url_path(dyntype, Some(ns));
        Self {
            client,
            request: Request::new(url),
            namespace: Some(ns.to_string()),
            _phantom: std::iter::empty(),
        }
    }

    /// Namespaced resource within the default namespace
    ///
    /// This function accepts `K::DynamicType` so it can be used with dynamic resources.
    ///
    /// Unless configured explicitly, the default namespace is either "default"
    /// out of cluster, or the service account's namespace in cluster.
    pub fn default_namespaced_with(client: Client, dyntype: &K::DynamicType) -> Self {
        let ns = client.default_ns().to_string();
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
    pub fn all(client: Client) -> Self {
        Self::all_with(client, &K::DynamicType::default())
    }

    /// Namespaced resource within a given namespace
    pub fn namespaced(client: Client, ns: &str) -> Self {
        Self::namespaced_with(client, ns, &K::DynamicType::default())
    }

    /// Namespaced resource within the default namespace
    ///
    /// Unless configured explicitly, the default namespace is either "default"
    /// out of cluster, or the service account's namespace in cluster.
    pub fn default_namespaced(client: Client) -> Self {
        Self::default_namespaced_with(client, &K::DynamicType::default())
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
            _phantom,
        } = self;
        f.debug_struct("Api")
            .field("request", &request)
            .field("client", &"...")
            .field("namespace", &namespace)
            .finish()
    }
}
