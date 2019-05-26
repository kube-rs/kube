#![allow(non_snake_case)]

use std::fmt::Debug;
use serde::{Deserialize};

use crate::api::{
    metadata::Metadata,
};
//use crate::{Result, Error};



// -------------------------------------------------------

/// ApiError for when things fail
///
/// This can be parsed into as an error handling fallback. Needed for `WatchEvent`;
/// It's quite commont to get a `410 Gone` when the resourceVersion is too old.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApiError {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    code: u16,
}

/// A raw event returned from a watch query
///
/// Note that a watch query returns many of these as newline separated json.
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<T, U> where
  T: Clone, U: Clone + Default,
{
    Added(Object<T, U>),
    Modified(Object<T, U>),
    Deleted(Object<T, U>),
    Error(ApiError),
}

impl<T, U> Debug for WatchEvent<T, U> where
   T: Clone, U: Clone + Default
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            WatchEvent::Added(_) =>  write!(f, "Added event"),
            WatchEvent::Modified(_) =>  write!(f, "Modified event"),
            WatchEvent::Deleted(_) =>  write!(f, "Deleted event"),
            WatchEvent::Error(e) =>  write!(f, "Error event: {:?}", e),
        }
    }
}

// -------------------------------------------------------

/// A generic kubernetes object
///
/// This is used instead of a full struct for `Deployment`, `Pod`, `Node`, `CRD`, ...
/// Kubernetes' API generally exposes core structs in this manner, but sometimes the
/// status, `U`, is not always present, and is occasionally passed as `Option<()>`.
///
/// The reasons we use this wrapper rather than the actual structs are:
/// - generic requirements on fields (need metadata) is impossible
/// - you cannot implement traits for objects you don't own => no addon traits to k8s-openapi
///
/// Thankfully, this generic setup works regardless, and the user is generally
/// unaware of the deception. Now it does require the user to pass explicit an Spec
/// and Status structs, which is slightly awkward.
///
/// This struct appears in `ObjectList` and `WatchEvent`, and when using a `Reflector`,
/// and is exposed as the values in `ObjectMap`.
#[derive(Deserialize, Serialize, Clone)]
pub struct Object<T, U> where
  T: Clone, U: Clone
{
    /// The version of the API
    ///
    /// Marked optional because it's not always present for items in a `ResourceList`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apiVersion: Option<String>,

    /// The name of the API
    ///
    /// Marked optional because it's not always present for items in a `ResourceList`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Resource metadata
    ///
    /// Contains information common to most resources about the Resource,
    /// including the object name, annotations, labels and more.
    pub metadata: Metadata,

    /// The Spec struct of a resource. I.e. `PodSpec`, `DeploymentSpec`, etc.
    ///
    /// This defines the desired state of the Resource as specified by the user.
    pub spec: T,

    /// The Status of a resource. I.e. `PotStatus`, `DeploymentStatus`, etc.
    ///
    /// This publishes the state of the Resource as observed by the controller.
    /// Internally passed as `Option<()>` when a status does not exist.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<U>,
}


/// A generic kubernetes object list
///
/// This is used instead of a full struct for `DeploymentList`, `PodList`, etc.
/// Kubernetes' API [always seem to expose list structs in this manner](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html?search=List).
///
/// Note that this is only used internally within reflectors and informers,
/// and is generally produced from list queries on an `Api`.
///
/// It should not be exposed outside this crate
#[derive(Deserialize)]
pub struct ObjectList<T> where
  T: Clone
{
    // NB: kind and apiVersion can be set here, but no need for it atm

    /// ListMeta - only really used for its resourceVersion
    ///
    /// See [ListMeta](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ListMeta.html)
    pub metadata: Metadata,

    /// The items we are actually interested in. In practice; T:= Resource<T,U>.
    #[serde(bound(deserialize = "Vec<T>: Deserialize<'de>"))]
    pub items: Vec<T>,
}

/*/// Generic post response object
///
/// Returned from patch / replace (incl. status)
#[derive(Deserialize, Serialize, Clone)]
//#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum PostResponse<T> where
    T: Clone
{
    Ok(T), // StatusCode::OK
    Created(T), // StatusCode::CREATED
    Error, // Unauthorized or other
}


/// Generic post response object
///
/// Returned from create new
#[derive(Deserialize, Serialize, Clone)]
pub enum CreateResponse<T> where
    T: Clone
{
    Ok(T), // StatusCode::OK
    Created(T), // StatusCode::CREATED
    Accepted(T), // StatusCode::ACCEPTED
    Error, // Unauthorized or other
}
// NB: 409 CONFLICT returned when already exists..


/// Generic response object
///
/// Returned from patch, get, watch style requests
#[derive(Deserialize, Serialize, Clone)]
//#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum Response<T> where
    T: Clone
{
    Ok(T),
    Error, // Unauthorized or other
}

// TODO: delete collection is weird - why would it give you meta::v1::Status?
//pub enum DeleteCollectionNamespacedDeploymentResponse {
//    OkStatus(crate::v1_13::apimachinery::pkg::apis::meta::v1::Status),
//    OkValue(crate::v1_13::api::apps::v1::Deployment),
//    Unauthorized,
//    Other,
//}*/
