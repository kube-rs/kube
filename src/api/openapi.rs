#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::api::{
    RawApi,
    Api,
};
use crate::client::{
    APIClient,
};

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionStatus as CrdStatus,
};
impl Api<CrdSpec, CrdStatus> {
    pub fn v1beta1CustomResourceDefinition(client: APIClient) -> Self {
        Api {
            api: RawApi::v1beta1CustomResourceDefinition(),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}


use k8s_openapi::api::core::v1::{NodeSpec, NodeStatus};
impl Api<NodeSpec, NodeStatus> {
    pub fn v1Node(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Node(),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}

use k8s_openapi::api::apps::v1::{DeploymentSpec, DeploymentStatus};
impl Api<DeploymentSpec, DeploymentStatus> {
    pub fn v1Deployment(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Deployment(),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}

use k8s_openapi::api::core::v1::{PodSpec, PodStatus};
impl Api<PodSpec, PodStatus> {
    pub fn v1Pod(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Pod(),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}
