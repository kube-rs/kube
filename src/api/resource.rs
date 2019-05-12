#![allow(non_snake_case)]

use std::fmt::Debug;
use serde::{Deserialize};

use crate::api::metadata::{
    Metadata,
};
use crate::{Result, Error};

/// Convenience converter into ApiResource
///
/// Allows people to not have to fill in all the fields of ApiResource manually.
/// Normally people just want the standard case with v1 urls and default prefixes.
/// The optional arg is always the namespace.
///
/// CustomResource purposefully ignored from this list.
#[derive(Debug, Clone)]
pub enum ResourceType {
    Nodes,
    Deploys(Option<String>),
    Pods(Option<String>),
}
impl Into<ApiResource> for ResourceType {
    fn into(self) -> ApiResource {
        match self {
            ResourceType::Nodes => ApiResource {
                group: "".into(),
                resource: "nodes".into(),
                version: "v1".into(),
                namespace: None,
                prefix: "api".into()
            },
            ResourceType::Deploys(ns) => ApiResource {
                group: "apps".into(),
                resource: "deployments".into(),
                version: "v1".into(),
                namespace: ns,
                prefix: "apis".into(),
            },
            ResourceType::Pods(ns) => ApiResource {
                group: "".into(),
                resource: "pods".into(),
                version: "v1".into(),
                namespace: ns,
                prefix: "api".into(),
            },
        }

    }
}

// -------------------------------------------------------

/// Resource representation from an API perspective
///
/// Used to construct the url for rest api calls.
#[derive(Clone, Debug)]
pub struct ApiResource {
    /// API Resource name
    pub resource: String,
    /// API Group
    pub group: String,
    /// Namespace the resources reside
    pub namespace: Option<String>,
    /// API version of the resource
    pub version: String,
    /// Name of the api prefix (api or apis typically)
    pub prefix: String,
}

impl Default for ApiResource {
    fn default() -> Self {
        Self {
            resource: "pods".into(), // had to pick something here
            namespace: None,
            group: "".into(),
            version: "v1".into(),
            prefix: "apis".into(), // seems most common
        }
    }
}
impl ToString for ApiResource {
    fn to_string(&self) -> String {
        let pref = if self.prefix == "" { "".into() } else { format!("{}/", self.prefix) };
        let g = if self.group == "" { "".into() } else { format!("{}/", self.group) };
        let v = if self.version == "" { "".into() } else { format!("{}/", self.version) };
        let n = if let Some(ns) = &self.namespace { format!("namespaces/{}/", ns) } else { "".into() };
        format!("/{prefix}{group}{version}{namespaces}{resource}?",
            prefix = pref,
            group = g,
            version = v,
            namespaces = n,
            resource = self.resource,
        )
    }
}

impl ApiResource {
    /// Create a list request to fully re-fetch the state
    pub fn list_all_resource_entries(&self) -> Result<http::Request<Vec<u8>>> {
        let urlstr = url::form_urlencoded::Serializer::new(self.to_string()).finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    /// Create a minimial list request to seed an initial resourceVersion
    pub fn list_zero_resource_entries(&self) -> Result<http::Request<Vec<u8>>> {
        let mut qp = url::form_urlencoded::Serializer::new(self.to_string());
        qp.append_pair("limit", "1"); // can't have 0..
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    /// Create watch request for a ApiResource at a given version
    pub fn watch_resource_entries_after(&self, ver: &str) -> Result<http::Request<Vec<u8>>> {
        let mut qp = url::form_urlencoded::Serializer::new(self.to_string());

        // Note that the timeoutSeconds is a fixed number regardless of activity
        // Set it to a sensible 10s for now. If less frequency is required, sleep.
        qp.append_pair("timeoutSeconds", "10");
        qp.append_pair("watch", "true");
        qp.append_pair("resourceVersion", ver);

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }
}



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
  T: Clone, U: Clone
{
    Added(Resource<T, U>),
    Modified(Resource<T, U>),
    Deleted(Resource<T, U>),
    Error(ApiError),
}

impl<T, U> Debug for WatchEvent<T, U> where
   T: Clone, U: Clone
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

/// A generic kubernetes resource
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
/// This struct appears in `ResourceList` and `WatchEvent`, and when using a `Reflector`,
/// it is exposed as the value of the `ResourceMap` to make it seem like a normal resouce object.
#[derive(Deserialize, Serialize, Clone)]
pub struct Resource<T, U> where
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
    pub status: U,
}


/// A generic kubernetes resource list
///
/// This is used instead of a full struct for `DeploymentList`, `PodList`, etc.
/// Kubernetes' API [always seem to expose list structs in this manner](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html?search=List).
///
/// Note that this is only used internally within reflectors and informers,
/// and is generally produced from list queries on an `ApiResource`.
///
/// It should not be exposed outside this crate
#[derive(Deserialize)]
pub struct ResourceList<T> where
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
