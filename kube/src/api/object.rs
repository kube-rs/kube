use crate::{
    api::metadata::{ListMeta, Meta, ObjectMeta, TypeMeta},
    error::ErrorResponse,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

/// A raw event returned from a watch query
///
/// Note that a watch query returns many of these as newline separated JSON.
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<K>
where
    K: Clone + Meta,
{
    /// Resource was added
    Added(K),
    /// Resource was modified
    Modified(K),
    /// Resource was deleted
    Deleted(K),
    /// Resource bookmark. `Bookmark` is a slimmed down `K` due to [#285](https://github.com/clux/kube-rs/issues/285).
    ///
    /// From [Watch bookmarks](https://kubernetes.io/docs/reference/using-api/api-concepts/#watch-bookmarks).
    ///
    /// NB: This became Beta first in Kubernetes 1.16.
    Bookmark(Bookmark),
    /// There was some kind of error
    Error(ErrorResponse),
}

impl<K> Debug for WatchEvent<K>
where
    K: Clone + Meta,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            WatchEvent::Added(_) => write!(f, "Added event"),
            WatchEvent::Modified(_) => write!(f, "Modified event"),
            WatchEvent::Deleted(_) => write!(f, "Deleted event"),
            WatchEvent::Bookmark(_) => write!(f, "Bookmark event"),
            WatchEvent::Error(e) => write!(f, "Error event: {:?}", e),
        }
    }
}

/// Slimed down K for [`WatchEvent::Bookmark`] due to [#285](https://github.com/clux/kube-rs/issues/285).
///
/// Can only be relied upon to have metadata with resource version.
/// Bookmarks contain apiVersion + kind + basically empty metadata.
#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    /// apiVersion + kind
    #[serde(flatten)]
    pub types: TypeMeta,

    /// Basically empty metadata
    pub metadata: BookmarkMeta,
}

/// Slimed down Metadata for WatchEvent::Bookmark
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkMeta {
    pub resource_version: String,
}

// -------------------------------------------------------

/// A standard Kubernetes object with `.spec` and `.status`.
///
/// This is a convenience struct provided for serialization/deserialization
/// It is not useful within the library anymore, because it can not easily implement
/// the [`k8s_openapi`] traits.
///
/// This is what Kubernetes maintainers tell you the world looks like.
/// It's.. generally true.
#[derive(Deserialize, Serialize, Clone)]
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
    /// A constructor like the one from kube-derive
    pub fn new<K: k8s_openapi::Resource>(name: &str, spec: P) -> Self {
        Self {
            types: TypeMeta {
                api_version: <K as k8s_openapi::Resource>::API_VERSION.to_string(),
                kind: <K as k8s_openapi::Resource>::KIND.to_string(),
            },
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                ..Default::default()
            },
            spec,
            status: None,
        }
    }
}

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
