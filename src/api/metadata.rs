#![allow(non_snake_case)]

use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct TypeMeta {
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
}


#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ListMeta {
    pub continue_: Option<String>,
    pub resourceVersion: Option<String>,
    pub selfLink: Option<String>,
}

/// Metadata that all persisted resources must have
///
/// This parses the relevant fields from `[ObjectMeta](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/apimachinery/pkg/apis/meta/v1/struct.ObjectMeta.html)`
/// Generally maps and vecs are moved out of their Options to avoid unnecessary boxing
/// because `xs.is_none()` is often functionally equivalent to `xs.is_empty()`.
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct ObjectMeta {
    /// The unique name (within namespace) for a resource
    ///
    /// This output from this from ResourceList calls is the empty string.
    #[serde(default)]
    pub name: String,

    /// The namespace (when it's namespaced) of the resouce where "" => "default"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,

    /// [Resource labels](http://kubernetes.io/docs/user-guide/labels)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub labels: BTreeMap<String, String>,

    /// [Resource annotations](http://kubernetes.io/docs/user-guide/annotations)
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,

    /// Kube internal version of the object to keep track of where to watch from
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resourceVersion: Option<String>,

    /// [Owner References](https://kubernetes.io/docs/concepts/workloads/controllers/garbage-collection/#owners-and-dependents)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ownerReferences: Vec<OwnerReference>,

    /// [Kubernetes generated UID](http://kubernetes.io/docs/user-guide/identifiers#uids)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,

    /// Sequence number representing the generation of this resource
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<f64>,

    /// Name prefix to be be used by kube if name is not provided
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generateName: Option<String>,

    /// List of initializers that have not yet acted on this object
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initializers: Option<Initializers>,

    /// List of finalizers to run before the object is deleted
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub finalizers: Vec<String>,
}

/// OwnerReference contains enough information to let you identify an owning object
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct OwnerReference {
    /// Whether the reference points to a managing controller
    #[serde(default)]
    pub controller: bool,
    /// Whether we can delete the owner before this is deleted
    #[serde(default)]
    pub blockOwnerDeletion: bool,
    /// Name of referent
    pub name: String,
    /// API version of the referent
    pub apiVersion: String,
    /// Kind of the referent
    pub kind: String,
}

/// Initializers tracks the progress of initialization
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Initializers {
    /// List of pending initializers that must execute before object is visible
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<Initializer>,

    /// Potential result of initializers
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Status>,
}

/// Information about an initializer that has not yet completed
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Initializer {
    /// Name of the process responsible for initializing the object
    pub name: String,
}

/// Status is a return value for calls that don't return other objects
#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Status {
    pub code: Option<i32>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
}
