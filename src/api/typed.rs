#![allow(non_snake_case)]

use either::Either;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::api::resource::{KubeObject, Object, ObjectList, WatchEvent};
use crate::api::{DeleteParams, ListParams, LogParams, PatchParams, PostParams, RawApi};
use crate::client::{APIClient, Status};
use crate::Result;

/// A typed Api variant that does not expose request internals
///
/// The upsides of working with this rather than `RawApi` direct are:
/// - easiers interface (no figuring out return types)
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
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + KubeObject,
{
    pub async fn get(&self, name: &str) -> Result<K> {
        let req = self.api.get(name)?;
        self.client.request::<K>(req).await
    }
    pub async fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.create(&pp, data)?;
        self.client.request::<K>(req).await
    }
    pub async fn delete(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>> {
        let req = self.api.delete(name, &dp)?;
        self.client.request_status::<K>(req).await
    }
    pub async fn list(&self, lp: &ListParams) -> Result<ObjectList<K>> {
        let req = self.api.list(&lp)?;
        self.client.request::<ObjectList<K>>(req).await
    }
    pub async fn delete_collection(
        &self,
        lp: &ListParams,
    ) -> Result<Either<ObjectList<K>, Status>> {
        let req = self.api.delete_collection(&lp)?;
        self.client.request_status::<ObjectList<K>>(req).await
    }
    pub async fn patch(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<K> {
        let req = self.api.patch(name, &pp, patch)?;
        self.client.request::<K>(req).await
    }
    pub async fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.replace(name, &pp, data)?;
        self.client.request::<K>(req).await
    }
    pub async fn watch(
        &self,
        lp: &ListParams,
        version: &str,
    ) -> Result<impl Stream<Item = WatchEvent<K>>> {
        let req = self.api.watch(&lp, &version)?;
        self.client
            .request_events::<WatchEvent<K>>(req)
            .await
            .map(|stream| stream.filter_map(|e| async move { e.ok() }))
    }

    pub async fn get_status(&self, name: &str) -> Result<K> {
        let req = self.api.get_status(name)?;
        self.client.request::<K>(req).await
    }
    pub async fn patch_status(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<K> {
        let req = self.api.patch_status(name, &pp, patch)?;
        self.client.request::<K>(req).await
    }
    pub async fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.api.replace_status(name, &pp, data)?;
        self.client.request::<K>(req).await
    }
}

/// Marker trait for objects that has logs
pub trait LoggingObject {}

impl<K> Api<K>
where
    K: Clone + DeserializeOwned + KubeObject + LoggingObject,
{
    pub async fn log(&self, name: &str, lp: &LogParams) -> Result<String> {
        let req = self.api.log(name, lp, false)?;
        Ok(self.client.request_text(req).await?)
    }

    pub async fn log_follow(&self, name: &str, lp: &LogParams) -> Result<impl Stream<Item = Result<Vec<u8>>>> {
        let req = self.api.log(name, lp, true)?;
        Ok(self.client.request_text_stream(req).await?)
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
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
    pub async fn get_scale(&self, name: &str) -> Result<Scale> {
        let req = self.api.get_scale(name)?;
        self.client.request::<Scale>(req).await
    }
    pub async fn patch_scale(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<Scale> {
        let req = self.api.patch_scale(name, &pp, patch)?;
        self.client.request::<Scale>(req).await
    }
    pub async fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<Scale> {
        let req = self.api.replace_scale(name, &pp, data)?;
        self.client.request::<Scale>(req).await
    }
}

/// Api Constructor for CRDs
///
/// Because it relies entirely on user definitions, this ctor does not rely on openapi.
impl<K> Api<K>
where
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
