use either::Either;
use futures::{Stream, StreamExt};
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;

use crate::{
    api::{DeleteParams, ListParams, Meta, ObjectList, PatchParams, PostParams, Resource, WatchEvent},
    client::{Client, Status},
    Result,
};

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
pub struct Api<K> {
    /// The request creator object
    pub(crate) api: Resource,
    /// The client to use (from this library)
    pub(crate) client: Client,
    /// Underlying Object unstored
    pub(crate) phantom: PhantomData<K>,
}

/// Expose same interface as Api for controlling scope/group/versions/ns
impl<K> Api<K>
where
    K: k8s_openapi::Resource,
{
    /// Cluster level resources, or resources viewed across all namespaces
    pub fn all(client: Client) -> Self {
        let api = Resource::all::<K>();
        Self {
            api,
            client,
            phantom: PhantomData,
        }
    }

    /// Namespaced resource within a given namespace
    pub fn namespaced(client: Client, ns: &str) -> Self {
        let api = Resource::namespaced::<K>(ns);
        Self {
            api,
            client,
            phantom: PhantomData,
        }
    }

    /// Consume self and return the [`Client`]
    pub fn into_client(self) -> Client {
        self.into()
    }
}

/// PUSH/PUT/POST/GET abstractions
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Meta,
{
    /// Get a named resource
    ///
    /// ```no_run
    /// use kube::{Api, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     let p: Pod = pods.get("blog").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get(&self, name: &str) -> Result<K> {
        let req = self.api.get(name)?;
        self.client.request::<K>(req).await
    }

    /// Get a list of resources
    ///
    /// You get use this to get everything, or a subset matching fields/labels, say:
    ///
    /// ```no_run
    /// use kube::{api::{Api, ListParams, Meta}, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     let lp = ListParams::default().labels("app=blog"); // for this app only
    ///     for p in pods.list(&lp).await? {
    ///         println!("Found Pod: {}", Meta::name(&p));
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list(&self, lp: &ListParams) -> Result<ObjectList<K>> {
        let req = self.api.list(&lp)?;
        self.client.request::<ObjectList<K>>(req).await
    }

    /// Create a resource
    ///
    /// This function requires a type that Serializes to `K`, which can be:
    /// 1. Raw string yaml
    ///   - easy to port from existing files
    ///   - error prone (run-time errors on typos due to failed serialize attempts)
    ///   - very error prone (can write invalid yaml)
    /// 2. An instance of the struct itself
    ///   - easy to instantiate for CRDs (you define the struct)
    ///   - dense to instantiate for k8s-openapi types (due to many optionals)
    ///   - compile-time safety
    ///   - but still possible to write invalid native types (validation at apiserver)
    /// 3. `serde_json::json!` macro instantiated `serde_json::Value`
    ///   - Tradeoff between the two
    ///   - Easy partially filling of native k8s-openapi types (most fields optional)
    ///   - Partial safety against runtime errors (at least you must write valid json)
    pub async fn create(&self, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Serialize,
    {
        let bytes = serde_json::to_vec(&data)?;
        let req = self.api.create(&pp, bytes)?;
        self.client.request::<K>(req).await
    }

    /// Delete a named resource
    ///
    /// When you get a `K` via `Left`, your delete has started.
    /// When you get a `Status` via `Right`, this should be a a 2XX style
    /// confirmation that the object being gone.
    ///
    /// 4XX and 5XX status types are returned as an `Err(kube::Error::Api)`
    ///
    /// ```no_run
    /// use kube::{api::{Api, DeleteParams}, Client};
    /// use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiexts;
    /// use apiexts::CustomResourceDefinition;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let crds: Api<CustomResourceDefinition> = Api::all(client);
    ///     crds.delete("foos.clux.dev", &DeleteParams::default()).await?
    ///         .map_left(|o| println!("Deleting CRD: {:?}", o.status))
    ///         .map_right(|s| println!("Deleted CRD: {:?}", s));
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>> {
        let req = self.api.delete(name, &dp)?;
        self.client.request_status::<K>(req).await
    }

    /// Delete a collection of resources
    ///
    /// When you get an `ObjectList<K>` via `Left`, your delete has started.
    /// When you get a `Status` via `Right`, this should be a a 2XX style
    /// confirmation that the object being gone.
    ///
    /// 4XX and 5XX status types are returned as an `Err(kube::Error::Api)`
    ///
    /// ```no_run
    /// use kube::{api::{Api, ListParams, Meta}, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     match pods.delete_collection(&ListParams::default()).await? {
    ///         either::Left(list) => {
    ///             let names: Vec<_> = list.iter().map(Meta::name).collect();
    ///             println!("Deleting collection of pods: {:?}", names);
    ///         },
    ///         either::Right(status) => {
    ///             println!("Deleted collection of pods: status={:?}", status);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_collection(&self, lp: &ListParams) -> Result<Either<ObjectList<K>, Status>> {
        let req = self.api.delete_collection(&lp)?;
        self.client.request_status::<ObjectList<K>>(req).await
    }

    /// Patch a resource a subset of its properties
    ///
    /// In all patch methods except PatchStrategy::Apply, you must set:
    /// `metadata.resourceVersion` to get k8s to accept the update.
    ///
    /// Thus to use these older patch methods you must first do a `get` then a `patch`.
    ///
    /// When using `PatchStrategy::Apply`, this restriction is not necessary,
    /// however, you **must** serialize your data using `serde_yaml`.
    /// NB: This is currently broken due to https://github.com/clux/kube-rs/issues/176
    pub async fn patch(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<K> {
        let req = self.api.patch(name, &pp, patch)?;
        self.client.request::<K>(req).await
    }

    /// Replace a resource entirely with a new one
    ///
    /// This is used just like `Api::create`, but with one additional instruction:
    /// You must set `metadata.resourceVersion` in the provided data because k8s
    /// will not accept an update unless you actually knew what the last version was.
    ///
    /// Thus, to use this function, you need to do a `get` then a `replace` with its result.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PostParams, Meta}, Client};
    /// use k8s_openapi::api::batch::v1::Job;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let mut j = jobs.get("baz").await?;
    ///     let j_new: Job = serde_json::from_value(serde_json::json!({
    ///         "apiVersion": "batch/v1",
    ///         "kind": "Job",
    ///         "metadata": {
    ///             "name": "baz",
    ///             "resourceVersion": Meta::resource_ver(&j),
    ///         },
    ///         "spec": {
    ///             "template": {
    ///                 "metadata": {
    ///                     "name": "empty-job-pod"
    ///                 },
    ///                 "spec": {
    ///                     "containers": [{
    ///                         "name": "empty",
    ///                         "image": "alpine:latest"
    ///                     }],
    ///                     "restartPolicy": "Never",
    ///                 }
    ///             }
    ///         }
    ///     }))?;
    ///     jobs.replace("baz", &PostParams::default(), &j_new).await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// Consider mutating the result of `api.get` rather than recreating it.
    pub async fn replace(&self, name: &str, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Serialize,
    {
        let bytes = serde_json::to_vec(&data)?;
        let req = self.api.replace(name, &pp, bytes)?;
        self.client.request::<K>(req).await
    }

    /// Watch a list of resources
    ///
    /// This returns a future that awaits the initial response,
    /// then you can stream the remaining buffered `WatchEvent` objects.
    ///
    /// ```no_run
    /// use kube::{api::{Api, ListParams, Meta, WatchEvent}, Client};
    /// use k8s_openapi::api::batch::v1::Job;
    /// use futures::StreamExt;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let lp = ListParams::default()
    ///         .fields("metadata.name=my_job")
    ///         .timeout(20); // upper bound of how long we watch for
    ///     let mut stream = jobs.watch(&lp, "0").await?.boxed();
    ///     while let Some(status) = stream.next().await {
    ///         match status {
    ///             WatchEvent::Added(s) => println!("Added {}", Meta::name(&s)),
    ///             WatchEvent::Modified(s) => println!("Modified: {}", Meta::name(&s)),
    ///             WatchEvent::Deleted(s) => println!("Deleted {}", Meta::name(&s)),
    ///             WatchEvent::Error(s) => println!("{}", s),
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn watch(&self, lp: &ListParams, version: &str) -> Result<impl Stream<Item = WatchEvent<K>>> {
        let req = self.api.watch(&lp, &version)?;
        self.client
            .request_events::<K>(req)
            .await
            .map(|stream| stream.filter_map(|e| async move { e.ok() }))
    }
}

impl<K> From<Api<K>> for Client {
    fn from(api: Api<K>) -> Self {
        api.client
    }
}
