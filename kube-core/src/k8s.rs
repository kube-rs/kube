//! Indirection layer for k8s-openapi / k8s-pb toggling.

#[cfg(feature = "openapi")]
pub use k8s_openapi::{
    api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus},
    api::core::v1::{ConfigMap, ObjectReference, Pod},
    apiextensions_apiserver::pkg::apis::apiextensions,
    apimachinery::pkg::apis::meta::v1::{
        LabelSelector, LabelSelectorRequirement, ListMeta, ManagedFieldsEntry, ObjectMeta, OwnerReference,
        Time,
    },
    ClusterResourceScope, Metadata, NamespaceResourceScope, Resource, ResourceScope, SubResourceScope,
};
#[cfg(feature = "pb")]
pub use k8s_pb::{
    api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus},
    api::core::v1::{ConfigMap, ObjectReference, Pod},
    apiextensions_apiserver::pkg::apis::apiextensions,
    apimachinery::pkg::apis::meta::v1::{
        LabelSelector, LabelSelectorRequirement, ListMeta, ManagedFieldsEntry, ObjectMeta, OwnerReference,
        Time,
    },
    ClusterResourceScope, Metadata, NamespaceResourceScope, Resource, ResourceScope, SubResourceScope,
};
