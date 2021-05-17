use bytes::Bytes;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::fmt::Debug;
use tracing::instrument;

use crate::{
    api::{Api, Patch, PatchParams, PostParams},
    client::Status,
    Result,
};

pub use kube_core::subresource::{EvictParams, LogParams};

#[cfg(feature = "ws")] pub use kube_core::subresource::AttachParams;

pub use k8s_openapi::api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus};

#[cfg(feature = "ws")] use crate::api::remote_command::AttachedProcess;

/// Methods for [scale subresource](https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#scale-subresource).
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
    /// Fetch the scale subresource
    #[instrument(skip(self), level = "trace")]
    pub async fn get_scale(&self, name: &str) -> Result<Scale> {
        let req = self.request.get_subresource("scale", name)?;
        self.client.request::<Scale>(req).await
    }

    /// Update the scale subresource
    #[instrument(skip(self), level = "trace")]
    pub async fn patch_scale<P: serde::Serialize + Debug>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<Scale> {
        let req = self.request.patch_subresource("scale", name, &pp, patch)?;
        self.client.request::<Scale>(req).await
    }

    /// Replace the scale subresource
    #[instrument(skip(self), level = "trace")]
    pub async fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<Scale> {
        let req = self.request.replace_subresource("scale", name, &pp, data)?;
        self.client.request::<Scale>(req).await
    }
}

// ----------------------------------------------------------------------------

// TODO: Replace examples with owned custom resources. Bad practice to write to owned objects
// These examples work, but the job controller will totally overwrite what we do.
/// Methods for [status subresource](https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#status-subresource).
impl<K> Api<K>
where
    K: DeserializeOwned,
{
    /// Get the named resource with a status subresource
    ///
    /// This actually returns the whole K, with metadata, and spec.
    #[instrument(skip(self), level = "trace")]
    pub async fn get_status(&self, name: &str) -> Result<K> {
        let req = self.request.get_subresource("status", name)?;
        self.client.request::<K>(req).await
    }

    /// Patch fields on the status object
    ///
    /// NB: Requires that the resource has a status subresource.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PatchParams, Patch}, Client};
    /// use k8s_openapi::api::batch::v1::Job;
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let mut j = jobs.get("baz").await?;
    ///     let pp = PatchParams::default(); // json merge patch
    ///     let data = serde_json::json!({
    ///         "status": {
    ///             "succeeded": 2
    ///         }
    ///     });
    ///     let o = jobs.patch_status("baz", &pp, &Patch::Merge(data)).await?;
    ///     assert_eq!(o.status.unwrap().succeeded, Some(2));
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), level = "trace")]
    pub async fn patch_status<P: serde::Serialize + Debug>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<K> {
        let req = self.request.patch_subresource("status", name, &pp, patch)?;
        self.client.request::<K>(req).await
    }

    /// Replace every field on the status object
    ///
    /// This works similarly to the [`Api::replace`] method, but `.spec` is ignored.
    /// You can leave out the `.spec` entirely from the serialized output.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PostParams}, Client};
    /// use k8s_openapi::api::batch::v1::{Job, JobStatus};
    /// #[tokio::main]
    /// async fn main() -> Result<(), kube::Error> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let mut o = jobs.get_status("baz").await?; // retrieve partial object
    ///     o.status = Some(JobStatus::default()); // update the job part
    ///     let pp = PostParams::default();
    ///     let o = jobs.replace_status("baz", &pp, serde_json::to_vec(&o)?).await?;
    ///     Ok(())
    /// }
    /// ```
    #[instrument(skip(self), level = "trace")]
    pub async fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.request.replace_subresource("status", name, &pp, data)?;
        self.client.request::<K>(req).await
    }
}

// ----------------------------------------------------------------------------
// Log subresource
// ----------------------------------------------------------------------------

#[test]
fn log_path() {
    use crate::api::{Request, Resource};
    use k8s_openapi::api::core::v1 as corev1;
    let lp = LogParams {
        container: Some("blah".into()),
        ..LogParams::default()
    };
    let url = corev1::Pod::url_path(&(), Some("ns"));
    let req = Request::new(url).logs("foo", &lp).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods/foo/log?&container=blah");
}

/// Marker trait for objects that has logs
pub trait Loggable {}

impl Loggable for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: DeserializeOwned + Loggable,
{
    /// Fetch logs as a string
    #[instrument(skip(self), level = "trace")]
    pub async fn logs(&self, name: &str, lp: &LogParams) -> Result<String> {
        let req = self.request.logs(name, lp)?;
        Ok(self.client.request_text(req).await?)
    }

    /// Fetch logs as a stream of bytes
    #[instrument(skip(self), level = "trace")]
    pub async fn log_stream(&self, name: &str, lp: &LogParams) -> Result<impl Stream<Item = Result<Bytes>>> {
        let req = self.request.logs(name, lp)?;
        Ok(self.client.request_text_stream(req).await?)
    }
}

// ----------------------------------------------------------------------------
// Eviction subresource
// ----------------------------------------------------------------------------

#[test]
fn evict_path() {
    use crate::api::{Request, Resource};
    use k8s_openapi::api::core::v1 as corev1;
    let ep = EvictParams::default();
    let url = corev1::Pod::url_path(&(), Some("ns"));
    let req = Request::new(url).evict("foo", &ep).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods/foo/eviction?");
}

/// Marker trait for objects that can be evicted
pub trait Evictable {}

impl Evictable for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: DeserializeOwned + Evictable,
{
    /// Create an eviction
    pub async fn evict(&self, name: &str, ep: &EvictParams) -> Result<Status> {
        let req = self.request.evict(name, ep)?;
        self.client.request::<Status>(req).await
    }
}

// ----------------------------------------------------------------------------
// Attach subresource
// ----------------------------------------------------------------------------

#[cfg(feature = "ws")]
#[test]
fn attach_path() {
    use crate::api::{Request, Resource};
    use k8s_openapi::api::core::v1 as corev1;
    let ap = AttachParams {
        container: Some("blah".into()),
        ..AttachParams::default()
    };
    let url = corev1::Pod::url_path(&(), Some("ns"));
    let req = Request::new(url).attach("foo", &ap).unwrap();
    assert_eq!(
        req.uri(),
        "/api/v1/namespaces/ns/pods/foo/attach?&stdout=true&stderr=true&container=blah"
    );
}

/// Marker trait for objects that has attach
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub trait Attachable {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Attachable for k8s_openapi::api::core::v1::Pod {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Attachable,
{
    /// Attach to pod
    #[instrument(skip(self), level = "trace")]
    pub async fn attach(&self, name: &str, ap: &AttachParams) -> Result<AttachedProcess> {
        let req = self.request.attach(name, ap)?;
        let stream = self.client.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }
}

// ----------------------------------------------------------------------------
// Exec subresource
// ----------------------------------------------------------------------------
#[cfg(feature = "ws")]
#[test]
fn exec_path() {
    use crate::api::{Request, Resource};
    use k8s_openapi::api::core::v1 as corev1;
    let ap = AttachParams {
        container: Some("blah".into()),
        ..AttachParams::default()
    };
    let url = corev1::Pod::url_path(&(), Some("ns"));
    let req = Request::new(url)
        .exec("foo", vec!["echo", "foo", "bar"], &ap)
        .unwrap();
    assert_eq!(
        req.uri(),
        "/api/v1/namespaces/ns/pods/foo/exec?&stdout=true&stderr=true&container=blah&command=echo&command=foo&command=bar"
    );
}

/// Marker trait for objects that has exec
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub trait Executable {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Executable for k8s_openapi::api::core::v1::Pod {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Executable,
{
    /// Execute a command in a pod
    #[instrument(skip(self), level = "trace")]
    pub async fn exec<I: Debug, T>(
        &self,
        name: &str,
        command: I,
        ap: &AttachParams,
    ) -> Result<AttachedProcess>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let req = self.request.exec(name, command, ap)?;
        let stream = self.client.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }
}
