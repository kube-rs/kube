use either::Either;
use futures::Stream;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

use crate::{api::Api, Error, Result};
use kube_core::{object::ObjectList, params::*, response::Status, ErrorResponse, WatchEvent};

/// PUSH/PUT/POST/GET abstractions
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Debug,
{
    /// Get a named resource
    ///
    /// ```no_run
    /// use kube::{Api, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     let p: Pod = pods.get("blog").await?;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// This function assumes that the object is expected to always exist, and returns [`Error`] if it does not.
    /// Consider using [`Api::get_opt`] if you need to handle missing objects.
    pub async fn get(&self, name: &str) -> Result<K> {
        let mut req = self.request.get(name).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get");
        self.client.request::<K>(req).await
    }

    /// [Get](`Api::get`) a named resource if it exists, returns [`None`] if it doesn't exist
    ///
    /// ```no_run
    /// use kube::{Api, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     if let Some(pod) = pods.get_opt("blog").await? {
    ///         // Pod was found
    ///     } else {
    ///         // Pod was not found
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_opt(&self, name: &str) -> Result<Option<K>> {
        match self.get(name).await {
            Ok(obj) => Ok(Some(obj)),
            Err(Error::Api(ErrorResponse { reason, .. })) if &reason == "NotFound" => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Get a list of resources
    ///
    /// You get use this to get everything, or a subset matching fields/labels, say:
    ///
    /// ```no_run
    /// use kube::{api::{Api, ListParams, ResourceExt}, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     let lp = ListParams::default().labels("app=blog"); // for this app only
    ///     for p in pods.list(&lp).await? {
    ///         println!("Found Pod: {}", p.name());
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn list(&self, lp: &ListParams) -> Result<ObjectList<K>> {
        let mut req = self.request.list(lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("list");
        self.client.request::<ObjectList<K>>(req).await
    }

    /// Create a resource
    ///
    /// This function requires a type that Serializes to `K`, which can be:
    /// 1. Raw string YAML
    ///     - easy to port from existing files
    ///     - error prone (run-time errors on typos due to failed serialize attempts)
    ///     - very error prone (can write invalid YAML)
    /// 2. An instance of the struct itself
    ///     - easy to instantiate for CRDs (you define the struct)
    ///     - dense to instantiate for [`k8s_openapi`] types (due to many optionals)
    ///     - compile-time safety
    ///     - but still possible to write invalid native types (validation at apiserver)
    /// 3. [`serde_json::json!`] macro instantiated [`serde_json::Value`]
    ///     - Tradeoff between the two
    ///     - Easy partially filling of native [`k8s_openapi`] types (most fields optional)
    ///     - Partial safety against runtime errors (at least you must write valid JSON)
    ///
    /// Note that this method cannot write to the status object (when it exists) of a resource.
    /// To set status objects please see [`Api::replace_status`] or [`Api::patch_status`].
    pub async fn create(&self, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Serialize,
    {
        let bytes = serde_json::to_vec(&data).map_err(Error::SerdeError)?;
        let mut req = self.request.create(pp, bytes).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create");
        self.client.request::<K>(req).await
    }

    /// Delete a named resource
    ///
    /// When you get a `K` via `Left`, your delete has started.
    /// When you get a `Status` via `Right`, this should be a a 2XX style
    /// confirmation that the object being gone.
    ///
    /// 4XX and 5XX status types are returned as an [`Err(kube_client::Error::Api)`](crate::Error::Api).
    ///
    /// ```no_run
    /// use kube::{api::{Api, DeleteParams}, Client};
    /// use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiexts;
    /// use apiexts::CustomResourceDefinition;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let crds: Api<CustomResourceDefinition> = Api::all(client);
    ///     crds.delete("foos.clux.dev", &DeleteParams::default()).await?
    ///         .map_left(|o| println!("Deleting CRD: {:?}", o.status))
    ///         .map_right(|s| println!("Deleted CRD: {:?}", s));
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete(&self, name: &str, dp: &DeleteParams) -> Result<Either<K, Status>> {
        let mut req = self.request.delete(name, dp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("delete");
        self.client.request_status::<K>(req).await
    }

    /// Delete a collection of resources
    ///
    /// When you get an `ObjectList<K>` via `Left`, your delete has started.
    /// When you get a `Status` via `Right`, this should be a a 2XX style
    /// confirmation that the object being gone.
    ///
    /// 4XX and 5XX status types are returned as an [`Err(kube_client::Error::Api)`](crate::Error::Api).
    ///
    /// ```no_run
    /// use kube::{api::{Api, DeleteParams, ListParams, ResourceExt}, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     match pods.delete_collection(&DeleteParams::default(), &ListParams::default()).await? {
    ///         either::Left(list) => {
    ///             let names: Vec<_> = list.iter().map(ResourceExt::name).collect();
    ///             println!("Deleting collection of pods: {:?}", names);
    ///         },
    ///         either::Right(status) => {
    ///             println!("Deleted collection of pods: status={:?}", status);
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_collection(
        &self,
        dp: &DeleteParams,
        lp: &ListParams,
    ) -> Result<Either<ObjectList<K>, Status>> {
        let mut req = self
            .request
            .delete_collection(dp, lp)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("delete_collection");
        self.client.request_status::<ObjectList<K>>(req).await
    }

    /// Patch a subset of a resource's properties
    ///
    /// Takes a [`Patch`] along with [`PatchParams`] for the call.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PatchParams, Patch, Resource}, Client};
    /// use k8s_openapi::api::core::v1::Pod;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let pods: Api<Pod> = Api::namespaced(client, "apps");
    ///     let patch = serde_json::json!({
    ///         "apiVersion": "v1",
    ///         "kind": "Pod",
    ///         "metadata": {
    ///             "name": "blog"
    ///         },
    ///         "spec": {
    ///             "activeDeadlineSeconds": 5
    ///         }
    ///     });
    ///     let params = PatchParams::apply("myapp");
    ///     let patch = Patch::Apply(&patch);
    ///     let o_patched = pods.patch("blog", &params, &patch).await?;
    ///     Ok(())
    /// }
    /// ```
    /// [`Patch`]: super::Patch
    /// [`PatchParams`]: super::PatchParams
    ///
    /// Note that this method cannot write to the status object (when it exists) of a resource.
    /// To set status objects please see [`Api::replace_status`] or [`Api::patch_status`].
    pub async fn patch<P: Serialize + Debug>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<K> {
        let mut req = self.request.patch(name, pp, patch).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("patch");
        self.client.request::<K>(req).await
    }

    /// Replace a resource entirely with a new one
    ///
    /// This is used just like [`Api::create`], but with one additional instruction:
    /// You must set `metadata.resourceVersion` in the provided data because k8s
    /// will not accept an update unless you actually knew what the last version was.
    ///
    /// Thus, to use this function, you need to do a `get` then a `replace` with its result.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PostParams, ResourceExt}, Client};
    /// use k8s_openapi::api::batch::v1::Job;
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let j = jobs.get("baz").await?;
    ///     let j_new: Job = serde_json::from_value(serde_json::json!({
    ///         "apiVersion": "batch/v1",
    ///         "kind": "Job",
    ///         "metadata": {
    ///             "name": "baz",
    ///             "resourceVersion": j.resource_version(),
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
    ///
    /// Note that this method cannot write to the status object (when it exists) of a resource.
    /// To set status objects please see [`Api::replace_status`] or [`Api::patch_status`].
    pub async fn replace(&self, name: &str, pp: &PostParams, data: &K) -> Result<K>
    where
        K: Serialize,
    {
        let bytes = serde_json::to_vec(&data).map_err(Error::SerdeError)?;
        let mut req = self
            .request
            .replace(name, pp, bytes)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("replace");
        self.client.request::<K>(req).await
    }

    /// Watch a list of resources
    ///
    /// This returns a future that awaits the initial response,
    /// then you can stream the remaining buffered `WatchEvent` objects.
    ///
    /// Note that a `watch` call can terminate for many reasons (even before the specified
    /// [`ListParams::timeout`] is triggered), and will have to be re-issued
    /// with the last seen resource version when or if it closes.
    ///
    /// Consider using a managed [`watcher`] to deal with automatic re-watches and error cases.
    ///
    /// ```no_run
    /// use kube::{api::{Api, ListParams, ResourceExt, WatchEvent}, Client};
    /// use k8s_openapi::api::batch::v1::Job;
    /// use futures::{StreamExt, TryStreamExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let lp = ListParams::default()
    ///         .fields("metadata.name=my_job")
    ///         .timeout(20); // upper bound of how long we watch for
    ///     let mut stream = jobs.watch(&lp, "0").await?.boxed();
    ///     while let Some(status) = stream.try_next().await? {
    ///         match status {
    ///             WatchEvent::Added(s) => println!("Added {}", s.name()),
    ///             WatchEvent::Modified(s) => println!("Modified: {}", s.name()),
    ///             WatchEvent::Deleted(s) => println!("Deleted {}", s.name()),
    ///             WatchEvent::Bookmark(s) => {},
    ///             WatchEvent::Error(s) => println!("{}", s),
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    /// [`ListParams::timeout`]: super::ListParams::timeout
    /// [`watcher`]: https://docs.rs/kube_runtime/*/kube_runtime/watcher/fn.watcher.html
    pub async fn watch(
        &self,
        lp: &ListParams,
        version: &str,
    ) -> Result<impl Stream<Item = Result<WatchEvent<K>>>> {
        let mut req = self.request.watch(lp, version).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("watch");
        self.client.request_events::<K>(req).await
    }
}
