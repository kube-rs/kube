//! Generic object and objectlist wrappers.
use crate::{
    api_resource::ApiResource,
    metadata::{ListMeta, ObjectMeta, TypeMeta},
    resource::Resource,
};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// A generic Kubernetes object list
///
/// This is used instead of a full struct for `DeploymentList`, `PodList`, etc.
/// Kubernetes' API [always seem to expose list structs in this manner](https://docs.rs/k8s-openapi/0.10.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html?search=List).
///
/// Note that this is only used internally within reflectors and informers,
/// and is generally produced from list/watch/delete collection queries on an [`Resource`](super::Resource).
///
/// This is almost equivalent to [`k8s_openapi::List<T>`](k8s_openapi::List), but iterable.
#[derive(Deserialize, Debug)]
pub struct ObjectList<T>
where
    T: Clone,
{
    // NB: kind and apiVersion can be set here, but no need for it atm
    /// ListMeta - only really used for its `resourceVersion`
    ///
    /// See [ListMeta](k8s_openapi::apimachinery::pkg::apis::meta::v1::ListMeta)
    pub metadata: ListMeta,

    /// The items we are actually interested in. In practice; `T := Resource<T,U>`.
    #[serde(bound(deserialize = "Vec<T>: Deserialize<'de>"))]
    pub items: Vec<T>,
}

impl<T: Clone> ObjectList<T> {
    /// `iter` returns an Iterator over the elements of this ObjectList
    ///
    /// # Example
    ///
    /// ```
    /// use kube::api::{ListMeta, ObjectList};
    ///
    /// let metadata: ListMeta = Default::default();
    /// let items = vec![1, 2, 3];
    /// let objectlist = ObjectList { metadata, items };
    ///
    /// let first = objectlist.iter().next();
    /// println!("First element: {:?}", first); // prints "First element: Some(1)"
    /// ```
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &T> + 'a {
        self.items.iter()
    }

    /// `iter_mut` returns an Iterator of mutable references to the elements of this ObjectList
    ///
    /// # Example
    ///
    /// ```
    /// use kube::api::{ObjectList, ListMeta};
    ///
    /// let metadata: ListMeta = Default::default();
    /// let items = vec![1, 2, 3];
    /// let mut objectlist = ObjectList { metadata, items };
    ///
    /// let mut first = objectlist.iter_mut().next();
    ///
    /// // Reassign the value in first
    /// if let Some(elem) = first {
    ///     *elem = 2;
    ///     println!("First element: {:?}", elem); // prints "First element: 2"
    /// }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut T> + 'a {
        self.items.iter_mut()
    }
}

impl<T: Clone> IntoIterator for ObjectList<T> {
    type IntoIter = ::std::vec::IntoIter<Self::Item>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a ObjectList<T> {
    type IntoIter = ::std::slice::Iter<'a, T>;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T: Clone> IntoIterator for &'a mut ObjectList<T> {
    type IntoIter = ::std::slice::IterMut<'a, T>;
    type Item = &'a mut T;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

// -------------------------------------------------------

/// A standard Kubernetes object with `.spec` and `.status`.
///
/// This is a convenience struct provided for serialization/deserialization.
/// It is slightly stricter than ['DynamicObject`] in that it enforces the spec/status convention,
/// and as such will not in general work with all api-discovered resources.
///
/// This can be used to tie existing resources to smaller, local struct variants to optimize for memory use.
/// E.g. if you are only interested in a few fields, but you store tons of them in memory with reflectors.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Object<P, U>
where
    P: Clone,
    U: Clone,
{
    /// The types field of an `Object`
    #[serde(flatten)]
    pub types: TypeMeta,

    /// Resource metadata
    ///
    /// Contains information common to most resources about the Resource,
    /// including the object name, annotations, labels and more.
    pub metadata: ObjectMeta,

    /// The Spec struct of a resource. I.e. `PodSpec`, `DeploymentSpec`, etc.
    ///
    /// This defines the desired state of the Resource as specified by the user.
    pub spec: P,

    /// The Status of a resource. I.e. `PodStatus`, `DeploymentStatus`, etc.
    ///
    /// This publishes the state of the Resource as observed by the controller.
    /// Use `U = NotUsed` when a status does not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<U>,
}

impl<P, U> Object<P, U>
where
    P: Clone,
    U: Clone,
{
    /// A constructor that takes Resource values from an `ApiResource`
    pub fn new(name: &str, ar: &ApiResource, spec: P) -> Self {
        Self {
            types: TypeMeta {
                api_version: ar.api_version.clone(),
                kind: ar.kind.clone(),
            },
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                ..Default::default()
            },
            spec,
            status: None,
        }
    }

    /// Attach a namespace to an Object
    pub fn within(mut self, ns: &str) -> Self {
        self.metadata.namespace = Some(ns.into());
        self
    }
}

impl<P, U> Resource for Object<P, U>
where
    P: Clone,
    U: Clone,
{
    type DynamicType = ApiResource;

    fn group(dt: &ApiResource) -> Cow<'_, str> {
        dt.group.as_str().into()
    }

    fn version(dt: &ApiResource) -> Cow<'_, str> {
        dt.version.as_str().into()
    }

    fn kind(dt: &ApiResource) -> Cow<'_, str> {
        dt.kind.as_str().into()
    }

    fn api_version(dt: &ApiResource) -> Cow<'_, str> {
        dt.api_version.as_str().into()
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}

/// Empty struct for when data should be discarded
///
/// Not using [`()`](https://doc.rust-lang.org/stable/std/primitive.unit.html), because serde's
/// [`Deserialize`](serde::Deserialize) `impl` is too strict.
#[derive(Clone, Deserialize, Serialize, Default, Debug)]
pub struct NotUsed {}


#[cfg(test)]
mod test {
    use super::{ApiResource, NotUsed, Object};
    #[test]
    fn simplified_k8s_object() {
        use k8s_openapi::api::core::v1::Pod;
        // Replacing heavy type k8s_openapi::api::core::v1::PodSpec with:
        #[derive(Clone)]
        struct PodSpecSimple {
            containers: Vec<ContainerSimple>,
        }
        #[derive(Clone)]
        struct ContainerSimple {
            image: String,
        }
        type PodSimple = Object<PodSpecSimple, NotUsed>;
        // by grabbing the ApiResource info from the Resource trait
        let ar = ApiResource::erase::<Pod>(&());
        assert_eq!(ar.group, "");
        assert_eq!(ar.kind, "Pod");
        let data = PodSpecSimple {
            containers: vec![ContainerSimple { image: "blog".into() }],
        };
        let mypod = PodSimple::new("blog", &ar, data).within("dev");
        assert_eq!(mypod.metadata.namespace.unwrap(), "dev");
        assert_eq!(mypod.metadata.name.unwrap(), "blog");
        assert_eq!(mypod.types.kind, "Pod");
        assert_eq!(mypod.types.api_version, "v1");
    }
}
