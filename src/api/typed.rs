#![allow(non_snake_case)]

use either::Either;
use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::{
    api::{
        object::{Metadata, ObjectList, WatchEvent},
        DeleteParams, ListParams, PatchParams, PostParams, RawApi,
    },
    client::{APIClient, Status},
    Result,
};

/// An easy Api interaction helper
///
/// The upsides of working with this rather than `RawApi` direct are:
/// - easiers serialization interface (no figuring out return types)
/// - client hidden within, less arguments
///
///
/// But the downsides are:
/// - k8s-openapi dependency required (feature'd)
/// - openapi types are unnecessarily heavy on Option use
/// - memory intensive structs because they contain the full data
/// - no control over requests (opinionated)
#[derive(Clone)]
pub struct Api<K> {
    /// The request creator object
    pub(crate) api: RawApi<K>,
    /// The client to use (from this library)
    pub(crate) client: APIClient,
    /// Underlying Object unstored
    pub(crate) phantom: PhantomData<K>,
}

/// Expose same interface as Api for controlling scope/group/versions/ns
#[cfg(feature = "openapi")]
impl<K> Api<K>
where
    K: k8s_openapi::Resource,
{
    pub fn new(client: APIClient) -> Self {
        let api = RawApi::<K>::global();
        Self {
            api,
            client,
            phantom: PhantomData,
        }
    }
}

/// PUSH/PUT/POST/GET abstractions
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Metadata,
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
