#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::api::{
    RawApi,
    Api,
    Object,
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
