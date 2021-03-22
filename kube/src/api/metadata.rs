pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use k8s_openapi::Metadata;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// An accessor trait for Metadata.
///
/// This is for a subset of Kubernetes type that do not end in `List`.
/// These types, using [`ObjectMeta`], SHOULD all have required properties:
/// - `.metadata`
/// - `.metadata.name`
///
/// And these optional properties:
/// - `.metadata.namespace`
/// - `.metadata.resource_version`
///
/// This avoids a bunch of the unnecessary unwrap mechanics for apps.
pub trait Meta {
    /// Type information for types that do not know their resource information at compile time.
    ///
    /// Types that know their metadata at compile time should select `Info = ()`.
    /// Types that require some information at runtime should select `Info`
    /// as type of this information.
    ///
    /// See [`DynamicObject`] for a valid implementation of non-k8s-openapi resources.
    type Info: Send + Sync + 'static;

    /// Returns kind of this object
    fn kind(f: &Self::Info) -> Cow<'_, str>;
    /// Returns group of this object
    fn group(f: &Self::Info) -> Cow<'_, str>;
    /// Returns version of this object
    fn version(f: &Self::Info) -> Cow<'_, str>;
    /// Returns apiVersion of this object
    fn api_version(f: &Self::Info) -> Cow<'_, str> {
        let group = Self::group(f);
        if group.is_empty() {
            return Self::version(f);
        }
        let mut group = group.into_owned();
        group.push('/');
        group.push_str(&Self::version(f));
        group.into()
    }
    /// Metadata that all persisted resources must have
    fn meta(&self) -> &ObjectMeta;
    /// The name of the resource
    fn name(&self) -> String;
    /// The namespace the resource is in
    fn namespace(&self) -> Option<String>;
    /// The resource version
    fn resource_ver(&self) -> Option<String>;
}

/// Implement accessor trait for any ObjectMeta-using Kubernetes Resource
impl<K> Meta for K
where
    K: Metadata<Ty = ObjectMeta>,
{
    type Info = ();

    fn kind<'a>(_: &()) -> Cow<'_, str> {
        K::KIND.into()
    }

    fn group<'a>(_: &()) -> Cow<'_, str> {
        K::GROUP.into()
    }

    fn version<'a>(_: &()) -> Cow<'_, str> {
        K::VERSION.into()
    }

    fn api_version(_: &()) -> Cow<'_, str> {
        K::API_VERSION.into()
    }

    fn meta(&self) -> &ObjectMeta {
        self.metadata()
    }

    fn name(&self) -> String {
        self.meta().name.clone().expect("kind has metadata.name")
    }

    fn resource_ver(&self) -> Option<String> {
        self.meta().resource_version.clone()
    }

    fn namespace(&self) -> Option<String> {
        self.meta().namespace.clone()
    }
}

/// A convenience struct for ad-hoc serialization.
///
/// Mostly useful for [`Object`](super::Object).
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}
