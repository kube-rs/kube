//! Metadata structs used in traits, lists, and dynamic objects.
use crate::{DynamicObject, Object};
pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// Type information that is flattened into every kubernetes object
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

/// A runtime accessor trait for TypeMeta
///
/// This trait can be thought of as a limited variant of the `Resource` trait that reads from runtime properties.
/// It cannot retrieve the plural, nor the scope of a resource and requires an `ApiResource` for this instead.
///
/// For static types is generally leans on the static information, but for dynamic types, it inspects the object.
pub trait TypeInfo {
    /// Get the `TypeMeta` of an object
    ///
    /// This is a safe `TypeMeta` getter for all object types
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


// static types always have type info
impl<K> TypeInfo for K
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

// dynamic types generally have typeinfo, but certain api endpoints can omit it
impl<P, U> TypeInfo for Object<P, U>
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

impl TypeInfo for DynamicObject {
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

// NB: we can implement ResourceExt for things that impl TypeInfo but not Resource
