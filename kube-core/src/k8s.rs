//! Indirection layer for k8s-openapi / k8s-pb toggling.

/// Re-export k8s-openapi types by default
#[cfg(feature = "openapi")]
pub use k8s_openapi::{
    api,
    api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus},
    api::core::v1::{ConfigMap, ObjectReference, Pod},
    apiextensions_apiserver::pkg::apis::apiextensions,
    apimachinery,
    apimachinery::pkg::apis::meta::v1::{
        LabelSelector, LabelSelectorRequirement, ListMeta, ManagedFieldsEntry, ObjectMeta, OwnerReference,
        Time,
    },
    ClusterResourceScope, Metadata, NamespaceResourceScope, Resource, ResourceScope, SubResourceScope,
};
/// Re-export k8s-pb types when only pb feature enabled
#[cfg(all(not(feature = "openapi"), feature = "pb"))]
pub use k8s_pb::{
    api,
    api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet},
    api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus},
    api::core::v1::{ConfigMap, ObjectReference, Pod},
    apiextensions_apiserver::pkg::apis::apiextensions,
    apimachinery,
    apimachinery::pkg::apis::meta::v1::{
        LabelSelector, LabelSelectorRequirement, ListMeta, ManagedFieldsEntry, ObjectMeta, OwnerReference,
        Time,
    },
    ClusterResourceScope, Metadata, NamespaceResourceScope, Resource, ResourceScope, SubResourceScope,
};

#[cfg(all(not(feature = "openapi"), not(feature = "pb")))]
compile_error!("At least one of openapi or pb feature must be enabled");
