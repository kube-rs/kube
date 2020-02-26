#![allow(non_snake_case)]

use either::Either;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::{
    api::{
        resource::{KubeObject, ObjectList, WatchEvent},
        DeleteParams, ListParams, PatchParams, PostParams, RawApi,
    },
    client::{APIClient, Status},
    Result,
};

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
    pub(crate) api: RawApi,
    /// The client to use (from this library)
    pub(crate) client: APIClient,
    /// Underlying Object unstored
    pub(crate) phantom: PhantomData<K>,
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

    pub async fn delete_collection(&self, lp: &ListParams) -> Result<Either<ObjectList<K>, Status>> {
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

    pub async fn watch(&self, lp: &ListParams, version: &str) -> Result<impl Stream<Item = WatchEvent<K>>> {
        let req = self.api.watch(&lp, &version)?;
        self.client
            .request_events::<WatchEvent<K>>(req)
            .await
            .map(|stream| stream.filter_map(|e| async move { e.ok() }))
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
