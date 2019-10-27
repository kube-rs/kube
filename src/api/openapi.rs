#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::api::{RawApi, Api, Object, Void};
use crate::api::typed::LoggingObject;
use crate::client::{
    APIClient,
};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionStatus as CrdStatus,
};
impl Api<Object<CrdSpec, CrdStatus>> {
    pub fn v1beta1CustomResourceDefinition(client: APIClient) -> Self {
        Api {
            api: RawApi::v1beta1CustomResourceDefinition(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::batch::v1beta1::{CronJobSpec, CronJobStatus};
impl Api<Object<CronJobSpec, CronJobStatus>> {
    pub fn v1beta1CronJob(client: APIClient) -> Self {
        Api {
            api: RawApi::v1beta1CronJob(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};
impl Api<Object<NodeSpec, NodeStatus>> {
    pub fn v1Node(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Node(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::apps::v1::{DeploymentSpec, DeploymentStatus};
impl Api<Object<DeploymentSpec, DeploymentStatus>> {
    pub fn v1Deployment(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Deployment(),
            client,
            phantom: PhantomData,
        }
    }
}
impl LoggingObject for Object<DeploymentSpec, DeploymentStatus> {}
impl LoggingObject for Object<DeploymentSpec, Void> {}

use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
impl Api<Object<PodSpec, PodStatus>> {
    pub fn v1Pod(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Pod(),
            client,
            phantom: PhantomData,
        }
    }
}
impl LoggingObject for Object<PodSpec, PodStatus> {}
impl LoggingObject for Object<PodSpec, Void> {}

use k8s_openapi::api::core::v1::{ServiceSpec, ServiceStatus};
impl Api<Object<ServiceSpec, ServiceStatus>> {
    pub fn v1Service(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Service(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::batch::v1::{JobSpec, JobStatus};
impl Api<Object<JobSpec, JobStatus>> {
    pub fn v1Job(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Job(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{NamespaceSpec, NamespaceStatus};
impl Api<Object<NamespaceSpec, NamespaceStatus>> {
    pub fn v1Namespace(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Namespace(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::apps::v1::{DaemonSetSpec, DaemonSetStatus};
impl Api<Object<DaemonSetSpec, DaemonSetStatus>> {
    pub fn v1DaemonSet(client: APIClient) -> Self {
        Api {
            api: RawApi::v1DaemonSet(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::apps::v1::{StatefulSetSpec, StatefulSetStatus};
impl Api<Object<StatefulSetSpec, StatefulSetStatus>> {
    pub fn v1StatefulSet(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Statefulset(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::apps::v1::{ReplicaSetSpec, ReplicaSetStatus};
impl Api<Object<ReplicaSetSpec, ReplicaSetStatus>> {
    pub fn v1ReplicaSet(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ReplicaSet(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{ReplicationControllerSpec, ReplicationControllerStatus};
impl Api<Object<ReplicationControllerSpec, ReplicationControllerStatus>> {
    pub fn v1ReplicationController(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ReplicationController(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{PersistentVolumeClaimSpec, PersistentVolumeClaimStatus};
impl Api<Object<PersistentVolumeClaimSpec, PersistentVolumeClaimStatus>> {
    pub fn v1PersistentVolumeClaim(client: APIClient) -> Self {
        Api {
            api: RawApi::v1PersistentVolumeClaim(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{PersistentVolumeSpec, PersistentVolumeStatus};
impl Api<Object<PersistentVolumeSpec, PersistentVolumeStatus>> {
    pub fn v1PersistentVolume(client: APIClient) -> Self {
        Api {
            api: RawApi::v1PersistentVolume(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::storage::v1::{VolumeAttachmentSpec, VolumeAttachmentStatus};
impl Api<Object<VolumeAttachmentSpec, VolumeAttachmentStatus>> {
    pub fn v1VolumeAttachment(client: APIClient) -> Self {
        Api {
            api: RawApi::v1VolumeAttachment(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{ResourceQuotaSpec, ResourceQuotaStatus};
impl Api<Object<ResourceQuotaSpec, ResourceQuotaStatus>> {
    pub fn v1ResourceQuota(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ResourceQuota(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::networking::v1::{NetworkPolicySpec};
impl Api<Object<NetworkPolicySpec, Void>> {
    pub fn v1NetworkPolicy(client: APIClient) -> Self {
        Api {
            api: RawApi::v1NetworkPolicy(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::autoscaling::v1::{HorizontalPodAutoscalerSpec, HorizontalPodAutoscalerStatus};
impl Api<Object<HorizontalPodAutoscalerSpec, HorizontalPodAutoscalerStatus>> {
    pub fn v1HorizontalPodAutoscaler(client: APIClient) -> Self {
        Api {
            api: RawApi::v1HorizontalPodAutoscaler(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::extensions::v1beta1::{IngressSpec, IngressStatus};
impl Api<Object<IngressSpec, IngressStatus>> {
    pub fn v1beta1Ingress(client: APIClient) -> Self {
        Api {
            api: RawApi::v1beta1Ingress(),
            client,
            phantom: PhantomData,
        }
    }
}
