use crate::{
    api::{DeleteParams, ListParams, Meta, ObjectList, PatchParams, PostParams, Resource, WatchEvent},
    client::{APIClient, Status},
    Result,
};
use either::Either;
use futures::{Stream, StreamExt};
use serde::{de::DeserializeOwned, Serialize};

/// An easy Api interaction helper
///
/// The upsides of working with this rather than a `Resource` directly are:
/// - easiers serialization interface (no figuring out return types)
/// - client hidden within, less arguments
///
/// But the downsides are:
/// - openapi types can take up a large amount of memory
/// - openapi types can be annoying to wrangle with their heavy Option use
/// - no control over requests (opinionated)
#[derive(Clone)]
pub struct Api {
    /// The request creator object
    pub(crate) api: Resource,
    /// The client to use (from this library)
    pub(crate) client: APIClient,
}

/// Expose same interface as Api for controlling scope/group/versions/ns
impl Api {
    /// Cluster level resources, or resources viewed across all namespaces
    pub fn all<K>(client: APIClient) -> Self
    where
        K: k8s_openapi::Resource,
    {
        let api = Resource::all::<K>();
        Self { api, client }
    }

    /// Namespaced resource within a given namespace
    pub fn namespaced<K>(client: APIClient, ns: &str) -> Self
    where
        K: k8s_openapi::Resource,
    {
        let api = Resource::namespaced::<K>(ns);
        Self { api, client }
    }
}

/// PUSH/PUT/POST/GET abstractions
impl Api {
    pub async fn get<K>(&self, name: &str) -> Result<K>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.get(name)?;
        self.client.request::<K>(req).await
    }

    pub async fn create<K>(&self, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Clone + DeserializeOwned + Meta + Serialize,
    {
        let bytes = serde_json::to_vec(data)?;
        let req = self.api.create(&pp, bytes)?;
        self.client.request::<K>(req).await
    }

    pub async fn delete<K>(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.delete(name, &dp)?;
        self.client.request_status::<K>(req).await
    }

    pub async fn list<K>(&self, lp: &ListParams) -> Result<ObjectList<K>>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.list(&lp)?;
        self.client.request::<ObjectList<K>>(req).await
    }

    pub async fn delete_collection<K>(&self, lp: &ListParams) -> Result<Either<ObjectList<K>, Status>>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.delete_collection(&lp)?;
        self.client.request_status::<ObjectList<K>>(req).await
    }

    pub async fn patch<K>(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<K>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.patch(name, &pp, patch)?;
        self.client.request::<K>(req).await
    }

    pub async fn replace<K>(&self, name: &str, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Clone + DeserializeOwned + Meta + Serialize,
    {
        let bytes = serde_json::to_vec(data)?;
        let req = self.api.replace(name, &pp, bytes)?;
        self.client.request::<K>(req).await
    }

    pub async fn watch<K>(&self, lp: &ListParams, version: &str) -> Result<impl Stream<Item = WatchEvent<K>>>
    where
        K: Clone + DeserializeOwned + Meta,
    {
        let req = self.api.watch(&lp, &version)?;
        self.client
            .request_events::<WatchEvent<K>>(req)
            .await
            .map(|stream| stream.filter_map(|e| async move { e.ok() }))
    }
}
