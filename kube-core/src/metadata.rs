//! Metadata structs used in traits, lists, and dynamic objects.
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};
use crate::{ApiResource, DynamicResourceScope, Object, Resource};

/// Type information that is flattened into every kubernetes object
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

/// A generic representation of any object with `ObjectMeta`.
///
/// It allows clients to get access to a particular `ObjectMeta`
/// schema without knowing the details of the version.
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PartialObjectMeta {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Standard object's metadata
    #[serde(default)]
    pub metadata: ObjectMeta,
}

impl From<PartialObjectMeta> for ObjectMeta {
    fn from(obj: PartialObjectMeta) -> Self {
        ObjectMeta { ..obj.metadata }
    }
}

impl Resource for PartialObjectMeta {
    type DynamicType = ApiResource;
    type Scope = DynamicResourceScope;

    fn kind(dt: &ApiResource) -> Cow<'_, str> {
        dt.kind.as_str().into()
    }

    fn group(dt: &ApiResource) -> Cow<'_, str> {
        dt.group.as_str().into()
    }

    fn version(dt: &ApiResource) -> Cow<'_, str> {
        dt.version.as_str().into()
    }

    fn plural(dt: &ApiResource) -> Cow<'_, str> {
        dt.plural.as_str().into()
    }
}


/// A runtime accessor trait for `TypeMeta`
///
/// This trait is a runtime subset of the `Resource` trait that can read the object directly.
/// It **cannot** retrieve the plural, **nor** the scope of a resource (which requires an `ApiResource`).
pub trait Inspect {
    /// Get the `TypeMeta` of an object
    ///
    /// This is a safe `TypeMeta` getter for all object types
    /// While it is generally safe to unwrap this option, do note that a few endpoints can elide it.
    fn types(&self) -> Option<TypeMeta>;

    /// Get the `TypeMeta` of an object that is guaranteed to have it
    ///
    /// Returns `TypeMeta` when exists, panics otherwise
    fn types_unchecked(&self) -> TypeMeta;

    /// Get the `kind` of an object
    fn kind(&self) -> Option<Cow<'_, str>>;
    /// Get the `apiVersion` of any object
    fn api_version(&self) -> Option<Cow<'_, str>>;
    /// Get a reference to the `ObjectMeta` of an object
    fn meta(&self) -> &ObjectMeta;
    /// Get a mutable reference to the `ObjectMeta` of an Object
    fn meta_mut(&mut self) -> &mut ObjectMeta;
}

// lean on static info on k8s_openapi generated types (safer than runtime lookups)
impl<K> Inspect for K
where
    K: k8s_openapi::Resource,
    K: k8s_openapi::Metadata<Ty = ObjectMeta>,
{
    fn types(&self) -> Option<TypeMeta> {
        Some(self.types_unchecked())
    }

    fn types_unchecked(&self) -> TypeMeta {
        TypeMeta {
            api_version: K::API_VERSION.into(),
            kind: K::KIND.into(),
        }
    }

    fn kind(&self) -> Option<Cow<'_, str>> {
        Some(K::KIND.into())
    }

    fn api_version(&self) -> Option<Cow<'_, str>> {
        Some(K::API_VERSION.into())
    }

    fn meta(&self) -> &ObjectMeta {
        self.metadata()
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        self.metadata_mut()
    }
}

// always lookup from object in dynamic cases
impl<P, U> Inspect for Object<P, U>
where
    P: Clone,
    U: Clone,
{
    fn types(&self) -> Option<TypeMeta> {
        self.types.clone()
    }

    fn types_unchecked(&self) -> TypeMeta {
        self.types.clone().unwrap()
    }

    fn kind(&self) -> Option<Cow<'_, str>> {
        self.types.as_ref().map(|t| Cow::Borrowed(t.kind.as_ref()))
    }

    fn api_version(&self) -> Option<Cow<'_, str>> {
        self.types.as_ref().map(|t| Cow::Borrowed(t.api_version.as_ref()))
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}

// always lookup from object in dynamic cases
impl Inspect for DynamicObject {
    fn types(&self) -> Option<TypeMeta> {
        self.types.clone()
    }

    fn types_unchecked(&self) -> TypeMeta {
        self.types.clone().unwrap()
    }

    fn kind(&self) -> Option<Cow<'_, str>> {
        self.types.as_ref().map(|t| Cow::Borrowed(t.kind.as_ref()))
    }

    fn api_version(&self) -> Option<Cow<'_, str>> {
        self.types.as_ref().map(|t| Cow::Borrowed(t.api_version.as_ref()))
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}
