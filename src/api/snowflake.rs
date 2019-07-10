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
