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
    /// Types that know their metadata at compile time should select `Family = ()`.
    /// Types that require some information at runtime should select `Family`
    /// as type of this information.
    type Family: Send + Sync + 'static;
    /// Returns kind of this object
    fn kind<'a>(f: &'a Self::Family) -> Cow<'a, str>;
    /// Returns group of this object
    fn group<'a>(f: &'a Self::Family) -> Cow<'a, str>;
    /// Returns version of this object
    fn version<'a>(f: &'a Self::Family) -> Cow<'a, str>;
    /// Returns apiVersion of this object
    fn api_version<'a>(f: &'a Self::Family) -> Cow<'a, str> {
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
    type Family = ();

    fn kind<'a>(_: &'a ()) -> Cow<'a, str> {
        K::KIND.into()
    }

    fn group<'a>(_: &'a ()) -> Cow<'a, str> {
        K::GROUP.into()
    }

    fn version<'a>(_: &'a ()) -> Cow<'a, str> {
        K::VERSION.into()
    }

    fn api_version<'a>(_: &'a ()) -> Cow<'a, str> {
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
