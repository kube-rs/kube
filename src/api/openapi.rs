#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::{
    api::{subresource::LoggingObject, Api, Object, RawApi, Void},
    client::APIClient,
};


/// Implement a named constructor on Api, with Spec and Status types
macro_rules! api_ctor {
    ( $name:tt, $Spec:ty, $Status:ty ) => {
        impl Api<Object<$Spec, $Status>> {
            pub fn $name(client: APIClient) -> Self {
                Api {
                    api: RawApi::$name(),
                    client,
                    phantom: PhantomData,
                }
            }
        }
    }
}

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec, CustomResourceDefinitionStatus as CrdStatus,
};
api_ctor!(v1beta1CustomResourceDefinition, CrdSpec, CrdStatus);

use k8s_openapi::api::batch::v1beta1::{CronJobSpec, CronJobStatus};
api_ctor!(v1beta1CronJob, CronJobSpec, CronJobStatus);

use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};
api_ctor!(v1Node, NodeSpec, NodeStatus);

use k8s_openapi::api::apps::v1::{DeploymentSpec, DeploymentStatus};
api_ctor!(v1Deployment, DeploymentSpec, DeploymentStatus);

impl LoggingObject for Object<DeploymentSpec, DeploymentStatus> {}
impl LoggingObject for Object<DeploymentSpec, Void> {}

use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
api_ctor!(v1Pod, PodSpec, PodStatus);

impl LoggingObject for Object<PodSpec, PodStatus> {}
impl LoggingObject for Object<PodSpec, Void> {}

use k8s_openapi::api::core::v1::{ServiceSpec, ServiceStatus};
api_ctor!(v1Service, ServiceSpec, ServiceStatus);

use k8s_openapi::api::batch::v1::{JobSpec, JobStatus};
api_ctor!(v1Job, JobSpec, JobStatus);

impl LoggingObject for Object<JobSpec, JobStatus> {}
impl LoggingObject for Object<JobSpec, Void> {}

use k8s_openapi::api::core::v1::{NamespaceSpec, NamespaceStatus};
api_ctor!(v1Namespace, NamespaceSpec, NamespaceStatus);

use k8s_openapi::api::apps::v1::{DaemonSetSpec, DaemonSetStatus};
api_ctor!(v1DaemonSet, DaemonSetSpec, DaemonSetStatus);

impl LoggingObject for Object<DaemonSetSpec, DaemonSetStatus> {}
impl LoggingObject for Object<DaemonSetSpec, Void> {}

use k8s_openapi::api::apps::v1::{StatefulSetSpec, StatefulSetStatus};
api_ctor!(v1StatefulSet, StatefulSetSpec, StatefulSetStatus);

impl LoggingObject for Object<StatefulSetSpec, StatefulSetStatus> {}
impl LoggingObject for Object<StatefulSetSpec, Void> {}

use k8s_openapi::api::apps::v1::{ReplicaSetSpec, ReplicaSetStatus};
api_ctor!(v1ReplicaSet, ReplicaSetSpec, ReplicaSetStatus);

impl LoggingObject for Object<ReplicaSetSpec, ReplicaSetStatus> {}
impl LoggingObject for Object<ReplicaSetSpec, Void> {}

use k8s_openapi::api::core::v1::{ReplicationControllerSpec, ReplicationControllerStatus};
api_ctor!(v1ReplicationController, ReplicationControllerSpec, ReplicationControllerStatus);

use k8s_openapi::api::core::v1::{PersistentVolumeClaimSpec, PersistentVolumeClaimStatus};
api_ctor!(v1PersistentVolumeClaim, PersistentVolumeClaimSpec, PersistentVolumeClaimStatus);

use k8s_openapi::api::core::v1::{PersistentVolumeSpec, PersistentVolumeStatus};
api_ctor!(v1PersistentVolume, PersistentVolumeSpec, PersistentVolumeStatus);

use k8s_openapi::api::storage::v1::{VolumeAttachmentSpec, VolumeAttachmentStatus};
api_ctor!(v1VolumeAttachment, VolumeAttachmentSpec, VolumeAttachmentStatus);

use k8s_openapi::api::core::v1::{ResourceQuotaSpec, ResourceQuotaStatus};
api_ctor!(v1ResourceQuota, ResourceQuotaSpec, ResourceQuotaStatus);

use k8s_openapi::api::networking::v1::NetworkPolicySpec;
api_ctor!(v1NetworkPolicy, NetworkPolicySpec, Void); // has no Status

use k8s_openapi::api::autoscaling::v1::{HorizontalPodAutoscalerSpec, HorizontalPodAutoscalerStatus};
api_ctor!(v1HorizontalPodAutoscaler, HorizontalPodAutoscalerSpec, HorizontalPodAutoscalerStatus);

use k8s_openapi::api::extensions::v1beta1::{IngressSpec, IngressStatus};
api_ctor!(v1beta1Ingress, IngressSpec, IngressStatus);

use k8s_openapi::api::authorization::v1::{SelfSubjectRulesReviewSpec, SubjectRulesReviewStatus};
api_ctor!(v1SelfSubjectRulesReview, SelfSubjectRulesReviewSpec, SubjectRulesReviewStatus);
