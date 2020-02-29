#![allow(non_snake_case)]
use crate::{
    api::metadata::{ListMeta, ObjectMeta, TypeMeta},
    ErrorResponse,
};
use serde::Deserialize;
use std::fmt::Debug;


/// Accessor trait needed to build higher level abstractions on kubernetes objects
///
/// Slight mirror of k8s_openapi::Metadata to avoid a hard dependency
/// Note that their trait does not require Metadata existence, but ours does.
#[cfg(not(feature = "openapi"))]
pub trait Metadata {
    /// The metadata type (typically ObjectMeta, but sometimes ListMeta)
    type Ty;
    /// Every object must have metadata
    ///
    /// But to match k8s_openapi::Metadata, we pretend it's optional
    fn metadata(&self) -> Option<&Self::Ty>;
}

/// Make sure they are have similar use cases
#[cfg(feature = "openapi")]
pub use k8s_openapi::Metadata;

pub trait MetaContent : Metadata {
    fn resource_ver(&self) -> Option<&String>;
}

#[cfg(not(feature = "openapi"))]
impl<K> MetaContent for K
where K: Metadata<Ty=ObjectMeta>
{
    fn resource_ver(&self) -> Option<&String> {
        self.metadata().expect("all types have metadata").resourceVersion.as_ref()
    }
}

#[cfg(feature = "openapi")]
impl<K> MetaContent for K
where K: k8s_openapi::Metadata<Ty=k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta>
{
    fn resource_ver(&self) -> Option<&String> {
        self.metadata().expect("all useful k8s_openapi types have metadata").resource_version.as_ref()
    }
}

/// A raw event returned from a watch query
///
/// Note that a watch query returns many of these as newline separated json.
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<K>
where
    K: Clone + Metadata,
{
    Added(K),
    Modified(K),
    Deleted(K),
    Error(ErrorResponse),
}

impl<K> Debug for WatchEvent<K>
where
    K: Clone + Metadata,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            WatchEvent::Added(_) => write!(f, "Added event"),
            WatchEvent::Modified(_) => write!(f, "Modified event"),
            WatchEvent::Deleted(_) => write!(f, "Deleted event"),
            WatchEvent::Error(e) => write!(f, "Error event: {:?}", e),
        }
    }
}

// -------------------------------------------------------

/// A standard kubernetes object with .spec and .status
///
/// This struct appears in `ObjectList` and `WatchEvent`, and when using a `Reflector`,
/// and is exposed as the values in `ObjectMap`.
#[derive(Deserialize, Serialize, Clone)]
pub struct Object<P, U>
where
    P: Clone,
    U: Clone,
{
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

    /// The Status of a resource. I.e. `PotStatus`, `DeploymentStatus`, etc.
    ///
    /// This publishes the state of the Resource as observed by the controller.
    /// Internally passed as `Option<()>` when a status does not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<U>,
}

/// Blanked implementation for standard objects that can use Object
impl<P, U> Metadata for Object<P, U>
where
    P: Clone,
    U: Clone,
    // TODO: only require Resource if in openapi cfg
    Object<P, U>: k8s_openapi::Resource
{
    type Ty = ObjectMeta;
    fn metadata(&self) -> Option<&ObjectMeta> {
        Some(&self.metadata)
    }
}

/// A generic kubernetes object list
///
/// This is used instead of a full struct for `DeploymentList`, `PodList`, etc.
/// Kubernetes' API [always seem to expose list structs in this manner](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html?search=List).
///
/// Note that this is only used internally within reflectors and informers,
/// and is generally produced from list/watch/delete collection queries on an `RawApi`.
#[derive(Deserialize)]
pub struct ObjectList<T>
where
    T: Clone,
{
    // NB: kind and apiVersion can be set here, but no need for it atm
    /// ListMeta - only really used for its resourceVersion
    ///
    /// See [ListMeta](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ListMeta.html)
    pub metadata: ListMeta,

    /// The items we are actually interested in. In practice; T:= Resource<T,U>.
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
