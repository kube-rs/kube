use bytes::Bytes;
use futures::Stream;
use serde::de::DeserializeOwned;
use std::fmt::Debug;

use crate::{
    api::{Api, Patch, PatchParams, PostParams},
    Error, Result,
};

use kube_core::response::Status;
pub use kube_core::subresource::{EvictParams, LogParams};

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub use kube_core::subresource::AttachParams;

pub use k8s_openapi::api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus};

#[cfg(feature = "ws")] use crate::api::portforward::Portforwarder;
#[cfg(feature = "ws")] use crate::api::remote_command::AttachedProcess;

/// Methods for [scale subresource](https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#scale-subresource).
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
    /// Fetch the scale subresource
    pub async fn get_scale(&self, name: &str) -> Result<Scale> {
        let mut req = self
            .request
            .get_subresource("scale", name)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get_scale");
        self.client.request::<Scale>(req).await
    }

    /// Update the scale subresource
    pub async fn patch_scale<P: serde::Serialize + Debug>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<Scale> {
        let mut req = self
            .request
            .patch_subresource("scale", name, pp, patch)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("patch_scale");
        self.client.request::<Scale>(req).await
    }

    /// Replace the scale subresource
    pub async fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<Scale> {
        let mut req = self
            .request
            .replace_subresource("scale", name, pp, data)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("replace_scale");
        self.client.request::<Scale>(req).await
    }
}

/// Arbitrary subresources
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Debug,
{
    /// Display one or many sub-resources.
    pub async fn get_subresource(&self, subresource_name: &str, name: &str) -> Result<K> {
        let mut req = self
            .request
            .get_subresource(subresource_name, name)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get_subresource");
        self.client.request::<K>(req).await
    }

    /// Create an instance of the subresource
    pub async fn create_subresource<T>(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut req = self
            .request
            .create_subresource(subresource_name, name, pp, data)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create_subresource");
        self.client.request::<T>(req).await
    }

    /// Patch an instance of the subresource
    pub async fn patch_subresource<P: serde::Serialize + Debug>(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<K> {
        let mut req = self
            .request
            .patch_subresource(subresource_name, name, pp, patch)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("patch_subresource");
        self.client.request::<K>(req).await
    }

    /// Replace an instance of the subresource
    pub async fn replace_subresource(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<K> {
        let mut req = self
            .request
            .replace_subresource(subresource_name, name, pp, data)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("replace_subresource");
        self.client.request::<K>(req).await
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
    pub async fn get_status(&self, name: &str) -> Result<K> {
        let mut req = self
            .request
            .get_subresource("status", name)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("get_status");
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
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    pub async fn patch_status<P: serde::Serialize + Debug>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<K> {
        let mut req = self
            .request
            .patch_subresource("status", name, pp, patch)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("patch_status");
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
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let jobs: Api<Job> = Api::namespaced(client, "apps");
    ///     let mut o = jobs.get_status("baz").await?; // retrieve partial object
    ///     o.status = Some(JobStatus::default()); // update the job part
    ///     let pp = PostParams::default();
    ///     let o = jobs.replace_status("baz", &pp, serde_json::to_vec(&o)?).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let mut req = self
            .request
            .replace_subresource("status", name, pp, data)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("replace_status");
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
pub trait Log {}

impl Log for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: DeserializeOwned + Log,
{
    /// Fetch logs as a string
    pub async fn logs(&self, name: &str, lp: &LogParams) -> Result<String> {
        let mut req = self.request.logs(name, lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("logs");
        self.client.request_text(req).await
    }

    /// Fetch logs as a stream of bytes
    pub async fn log_stream(&self, name: &str, lp: &LogParams) -> Result<impl Stream<Item = Result<Bytes>>> {
        let mut req = self.request.logs(name, lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("log_stream");
        self.client.request_text_stream(req).await
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
pub trait Evict {}

impl Evict for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: DeserializeOwned + Evict,
{
    /// Create an eviction
    pub async fn evict(&self, name: &str, ep: &EvictParams) -> Result<Status> {
        let mut req = self.request.evict(name, ep).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("evict");
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
pub trait Attach {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Attach for k8s_openapi::api::core::v1::Pod {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Attach,
{
    /// Attach to pod
    pub async fn attach(&self, name: &str, ap: &AttachParams) -> Result<AttachedProcess> {
        let mut req = self.request.attach(name, ap).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("attach");
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
pub trait Execute {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Execute for k8s_openapi::api::core::v1::Pod {}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Execute,
{
    /// Execute a command in a pod
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
        let mut req = self
            .request
            .exec(name, command, ap)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("exec");
        let stream = self.client.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }
}

// ----------------------------------------------------------------------------
// Portforward subresource
// ----------------------------------------------------------------------------
#[cfg(feature = "ws")]
#[test]
fn portforward_path() {
    use crate::api::{Request, Resource};
    use k8s_openapi::api::core::v1 as corev1;
    let url = corev1::Pod::url_path(&(), Some("ns"));
    let req = Request::new(url).portforward("foo", &[80, 1234]).unwrap();
    assert_eq!(
        req.uri(),
        "/api/v1/namespaces/ns/pods/foo/portforward?&ports=80%2C1234"
    );
}

/// Marker trait for objects that has portforward
#[cfg(feature = "ws")]
pub trait Portforward {}

#[cfg(feature = "ws")]
impl Portforward for k8s_openapi::api::core::v1::Pod {}

#[cfg(feature = "ws")]
impl<K> Api<K>
where
    K: Clone + DeserializeOwned + Portforward,
{
    /// Forward ports of a pod
    pub async fn portforward(&self, name: &str, ports: &[u16]) -> Result<Portforwarder> {
        let req = self
            .request
            .portforward(name, ports)
            .map_err(Error::BuildRequest)?;
        let stream = self.client.connect(req).await?;
        Ok(Portforwarder::new(stream, ports))
    }
}
