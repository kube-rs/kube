#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::api::{
    RawApi,
    Api,
    Object,
    Log
};
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

impl Log for Api<Object<PodSpec, PodStatus>> {}

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
