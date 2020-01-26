//! Snowflake types that do not follow the Object<P, U> kube standard
#![allow(non_snake_case, non_camel_case_types)]

use std::collections::BTreeMap;
use core::marker::PhantomData;
use crate::client::APIClient;
use crate::api::{
    RawApi, Api, KubeObject,
    ObjectMeta
};


use k8s_openapi::apimachinery::pkg::apis::meta::v1::{MicroTime, Time};
use k8s_openapi::api::core::v1::{EventSeries, ObjectReference, EventSource};

/// Janky Event object
///
/// https://kubernetes.io/docs/reference/federation/v1/definitions/#_v1_event
#[derive(Deserialize, Serialize, Clone)]
pub struct v1Event {

    pub metadata: ObjectMeta,

    // Require properties
    pub involvedObject: ObjectReference,

    // These are still often set to empty string..
    #[serde(default)]
    pub reportingComponent: String,
    #[serde(default)]
    pub reportingInstance: String,

    // Properties that always seem present but arent required:

    #[serde(default)]
    pub message: String,

    #[serde(default)]
    pub reason: String,

    #[serde(default)]
    pub count: i32,

    #[serde(default, rename = "type")]
    pub type_: String,


    // Mist optionals gunk from openapi
    pub action: Option<String>,
    pub eventTime: Option<MicroTime>,
    pub firstTimestamp: Option<Time>,
    pub lastTimestamp: Option<Time>,
    pub related: Option<ObjectReference>,
    pub series: Option<EventSeries>,
    pub source: Option<EventSource>,

}

// Special case implementation so we can make Informer<Event> etc.
impl KubeObject for v1Event {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1Event> {
    pub fn v1Event(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Event(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::ByteString;
/// Secret object
///
/// https://kubernetes.io/docs/reference/federation/v1/definitions/#_v1_secret
#[derive(Deserialize, Serialize, Clone)]
pub struct v1Secret {
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, ByteString>,

    pub metadata: ObjectMeta,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub stringData: BTreeMap<String, String>,

    #[serde(default, rename = "type")]
    pub type_: Option<String>,
}


// Special case implementation so we can make Informer<v1Secret> etc.
impl KubeObject for v1Secret {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1Secret> {
    pub fn v1Secret(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Secret(),
            client,
            phantom: PhantomData,
        }
    }
}

/// ConfigMap object
#[derive(Deserialize, Serialize, Clone)]
pub struct v1ConfigMap {
    pub metadata: ObjectMeta,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub binaryData: BTreeMap<String, ByteString>,

    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub data: BTreeMap<String, String>
}

impl KubeObject for v1ConfigMap {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1ConfigMap> {
    pub fn v1ConfigMap(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ConfigMap(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::admissionregistration::v1beta1::ValidatingWebhook;

/// ValidatingWebhookConfiguration object
#[derive(Deserialize, Serialize, Clone)]
pub struct v1beta1ValidatingWebhookConfiguration {
    pub metadata: ObjectMeta,

    pub webhooks: Vec<ValidatingWebhook>
}

impl KubeObject for v1beta1ValidatingWebhookConfiguration {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1beta1ValidatingWebhookConfiguration> {
    pub fn v1beta1ValidatingWebhookConfiguration(client: APIClient) -> Self {
        Api {
            api: RawApi::v1beta1ValidatingWebhookConfiguration(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::rbac::v1::PolicyRule;
/// Role object
#[derive(Deserialize, Serialize, Clone)]
pub struct v1Role {
    pub metadata: ObjectMeta,
    pub rules: Vec<PolicyRule>,
}

impl KubeObject for v1Role {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1Role> {
    pub fn v1Role(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Role(),
            client,
            phantom: PhantomData,
        }
    }
}
/// ClusterRole object
#[derive(Deserialize, Serialize, Clone)]
pub struct v1ClusterRole {
    pub metadata: ObjectMeta,
    pub rules: Vec<PolicyRule>,
}

impl KubeObject for v1ClusterRole {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1ClusterRole> {
    pub fn v1ClusterRole(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ClusterRole(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::rbac::v1::{RoleRef, Subject};
/// Role Binding object
#[derive(Deserialize, Serialize, Clone)]
pub struct v1RoleBinding {
    pub metadata: ObjectMeta,
    pub roleRef: RoleRef,
    pub subjects: Vec<Subject>,
}

impl KubeObject for v1RoleBinding {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1RoleBinding> {
    pub fn v1RoleBinding(client: APIClient) -> Self {
        Api {
            api: RawApi::v1RoleBinding(),
            client,
            phantom: PhantomData,
        }
    }
}
/// Service Account object
///
/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#serviceaccount-v1-core
/// https://arnavion.github.io/k8s-openapi/v0.7.x/k8s_openapi/api/core/v1/struct.ServiceAccount.html
#[derive(Deserialize, Serialize, Clone)]
pub struct v1ServiceAccount {
    pub metadata: ObjectMeta,
    pub automountServiceAccountToken: bool,
    /// TODO: add remaining fields incomplete - atm, only here to allow listing
}

impl KubeObject for v1ServiceAccount {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1ServiceAccount> {
    pub fn v1ServiceAccount(client: APIClient) -> Self {
        Api {
            api: RawApi::v1ServiceAccount(),
            client,
            phantom: PhantomData,
        }
    }
}

use k8s_openapi::api::core::v1::{EndpointSubset};

/// Endpoint
/// https://kubernetes.io/docs/reference/generated/kubernetes-api/v1.17/#endpoints-v1-core
#[derive(Deserialize, Serialize, Clone)]
pub struct v1Endpoint {

    pub metadata: ObjectMeta,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subsets: Vec<EndpointSubset>,
}

impl KubeObject for v1Endpoint {
    fn meta(&self) -> &ObjectMeta { &self.metadata }
}

impl Api<v1Endpoint> {
    pub fn v1Endpoint(client: APIClient) -> Self {
        Api {
            api: RawApi::v1Endpoint(),
            client,
            phantom: PhantomData,
        }
    }
}
