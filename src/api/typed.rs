#![allow(non_snake_case)]

use either::{Either};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::api::{
    RawApi,
    PostParams,
    DeleteParams,
    ListParams,
};
use crate::api::resource::{
    ObjectList, Object, WatchEvent, KubeObject,
};
use crate::client::{
    APIClient,
    Status,
};
use crate::{Result};

/// A typed Api variant that does not expose request internals
///
/// The upsides of working with this rather than `Api` direct are:
/// - super easy interface (no figuring out return types)
/// - openapi types for free
///
/// But the downsides are:
/// - k8s-openapi dependency required (behind feature)
/// - openapi types are unnecessarily heavy on Option use
/// - memory intensive structs because they contain the full data
/// - no control over requests (opinionated)
#[derive(Clone)]
pub struct Api<K> {
    /// The request creator object
    pub(in crate::api) api: RawApi,
    /// The client to use (from this library)
    pub(in crate::api) client: APIClient,
    /// sPec and statUs structs
    pub(in crate::api) phantom: PhantomData<K>,
}

/// Expose same interface as Api for controlling scope/group/versions/ns
impl<K> Api<K> {
    pub fn within(mut self, ns: &str) -> Self {
        self.api = self.api.within(ns);
        self
    }
    pub fn group(mut self, group: &str) -> Self {
        self.api = self.api.group(group);
        self
    }
    pub fn version(mut self, version: &str) -> Self {
        self.api = self.api.version(version);
        self
    }
}

/// PUSH/PUT/POST/GET abstractions
impl<K> Api<K> where
    K: Clone + DeserializeOwned + KubeObject,
{
    pub fn get(&self, name: &str) -> Result<K> {
        let req = self.api.get(name)?;
        self.client.request::<K>(req)
    }
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.create(&pp, data)?;
        self.client.request::<K>(req)
    }
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>> {
        let req = self.api.delete(name, &dp)?;
        self.client.request_status::<K>(req)
    }
    pub fn list(&self, lp: &ListParams) -> Result<ObjectList<K>> {
        let req = self.api.list(&lp)?;
        self.client.request::<ObjectList<K>>(req)
    }
    pub fn delete_collection(&self, lp: &ListParams) -> Result<Either<ObjectList<K>, Status>> {
        let req = self.api.delete_collection(&lp)?;
        self.client.request_status::<ObjectList<K>>(req)
    }
    pub fn patch(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<K> {
        let req = self.api.patch(name, &pp, patch)?;
        self.client.request::<K>(req)
    }
    pub fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.replace(name, &pp, data)?;
        self.client.request::<K>(req)
    }
    pub fn watch(&self, lp: &ListParams, version: &str) -> Result<Vec<WatchEvent<K>>> {
        let req = self.api.watch(&lp, &version)?;
        self.client.request_events::<WatchEvent<K>>(req)
    }
    pub fn get_status(&self, name: &str) -> Result<K> {
        let req = self.api.get_status(name)?;
        self.client.request::<K>(req)
    }
    pub fn patch_status(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<K> {
        let req = self.api.patch_status(name, &pp, patch)?;
        self.client.request::<K>(req)
    }
    pub fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.replace_status(name, &pp, data)?;
        self.client.request::<K>(req)
    }
}

/// Scale spec from api::autoscaling::v1
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ScaleSpec {
    pub replicas: Option<i32>,
}
/// Scale status from api::autoscaling::v1
#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct ScaleStatus {
    pub replicas: i32,
    pub selector: Option<String>,
}
pub type Scale = Object<ScaleSpec, ScaleStatus>;

/// Scale subresource
///
/// https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#scale-subresource
impl<K> Api<K> where
    K: Clone + DeserializeOwned,
{
    pub fn get_scale(&self, name: &str) -> Result<Scale> {
        let req = self.api.get_scale(name)?;
        self.client.request::<Scale>(req)
    }
    pub fn patch_scale(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<Scale> {
        let req = self.api.patch_scale(name, &pp, patch)?;
        self.client.request::<Scale>(req)
    }
    pub fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<Scale> {
        let req = self.api.replace_scale(name, &pp, data)?;
        self.client.request::<Scale>(req)
    }
}

/// Api Constructor for CRDs
///
/// This is the only native object that does not rely on openapi.
/// But that's only because it's still generic and relies on user definitions.
impl<K> Api<K> where
    K: Clone + DeserializeOwned,
{
    pub fn customResource(client: APIClient, name: &str) -> Self {
        Self {
            api: RawApi::customResource(name),
            client,
            phantom: PhantomData,
        }
    }
}

// all other native impls in openapi.rs
