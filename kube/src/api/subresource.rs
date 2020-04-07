use bytes::Bytes;
use futures::Stream;
use serde::de::DeserializeOwned;

use crate::{
    api::{Api, PatchParams, PostParams, Resource},
    Error, Result,
};

pub use k8s_openapi::api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus};

/// Scale subresource
///
/// https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#scale-subresource
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
    /// Fetch the scale subresource
    pub async fn get_scale(&self, name: &str) -> Result<Scale> {
        let req = self.resource.get_scale(name)?;
        self.client.request::<Scale>(req).await
    }

    /// Update the scale subresource
    pub async fn patch_scale(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<Scale> {
        let req = self.resource.patch_scale(name, &pp, patch)?;
        self.client.request::<Scale>(req).await
    }

    /// Replace the scale subresource
    pub async fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<Scale> {
        let req = self.resource.replace_scale(name, &pp, data)?;
        self.client.request::<Scale>(req).await
    }
}

// ----------------------------------------------------------------------------

/// Status subresource
///
/// https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#status-subresource
/// TODO: Replace examples with owned custom resources. Bad practice to write to owned objects
/// These examples work, but the job controller will totally overwrite what we do.
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
    /// Get the named resource with a status subresource
    ///
    /// This actually returns the whole K, with metadata, and spec.
    pub async fn get_status(&self, name: &str) -> Result<K> {
        let req = self.resource.get_status(name)?;
        self.client.request::<K>(req).await
    }

    /// Patch fields on the status object
    ///
    /// NB: Requires that the resource has a status subresource.
    ///
    /// ```no_run
    /// use kube::{api::{Api, PatchParams}, Client};
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
    ///     let o = jobs.patch_status("baz", &pp, serde_json::to_vec(&data)?).await?;
    ///     assert_eq!(o.status.unwrap().succeeded, Some(2));
    ///     Ok(())
    /// }
    /// ```
    pub async fn patch_status(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<K> {
        let req = self.resource.patch_status(name, &pp, patch)?;
        self.client.request::<K>(req).await
    }

    /// Replace every field on the status object
    ///
    /// This works similarly to the `Api::replace` method, but .spec is ignored.
    /// You can leave out the .spec entirely from the serialized output.
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
    pub async fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<K> {
        let req = self.resource.replace_status(name, &pp, data)?;
        self.client.request::<K>(req).await
    }
}

// ----------------------------------------------------------------------------
// Log subresource
// ----------------------------------------------------------------------------

/// Params for logging
#[derive(Default, Clone, Debug)]
pub struct LogParams {
    /// The container for which to stream logs. Defaults to only container if there is one container in the pod.
    pub container: Option<String>,
    /// Follow the log stream of the pod. Defaults to false.
    pub follow: bool,
    /// If set, the number of bytes to read from the server before terminating the log output.
    /// This may not display a complete final line of logging, and may return slightly more or slightly less than the specified limit.
    pub limit_bytes: Option<i64>,
    /// If 'true', then the output is pretty printed.
    pub pretty: bool,
    /// Return previous terminated container logs. Defaults to false.
    pub previous: bool,
    /// A relative time in seconds before the current time from which to show logs.
    /// If this value precedes the time a pod was started, only logs since the pod start will be returned.
    /// If this value is in the future, no logs will be returned. Only one of sinceSeconds or sinceTime may be specified.
    pub since_seconds: Option<i64>,
    /// If set, the number of lines from the end of the logs to show.
    /// If not specified, logs are shown from the creation of the container or sinceSeconds or sinceTime
    pub tail_lines: Option<i64>,
    /// If true, add an RFC3339 or RFC3339Nano timestamp at the beginning of every line of log output. Defaults to false.
    pub timestamps: bool,
}

impl Resource {
    /// Get a pod logs
    pub fn logs(&self, name: &str, lp: &LogParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/" + "log?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        if let Some(container) = &lp.container {
            qp.append_pair("container", &container);
        }

        if lp.follow {
            qp.append_pair("follow", "true");
        }

        if let Some(lb) = &lp.limit_bytes {
            qp.append_pair("limitBytes", &lb.to_string());
        }

        if lp.pretty {
            qp.append_pair("pretty", "true");
        }

        if lp.previous {
            qp.append_pair("previous", "true");
        }

        if let Some(ss) = &lp.since_seconds {
            qp.append_pair("sinceSeconds", &ss.to_string());
        }

        if let Some(tl) = &lp.tail_lines {
            qp.append_pair("tailLines", &tl.to_string());
        }

        if lp.timestamps {
            qp.append_pair("timestamps", "true");
        }

        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }
}

#[test]
fn log_path() {
    use crate::api::Resource;
    use k8s_openapi::api::core::v1 as corev1;
    let r = Resource::namespaced::<corev1::Pod>("ns");
    let mut lp = LogParams::default();
    lp.container = Some("blah".into());
    let req = r.logs("foo", &lp).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods/foo/log?&container=blah");
}

/// Marker trait for objects that has logs
pub trait LoggingObject {}

impl LoggingObject for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: Clone + DeserializeOwned + LoggingObject,
{
    /// Fetch logs as a string
    pub async fn logs(&self, name: &str, lp: &LogParams) -> Result<String> {
        let req = self.resource.logs(name, lp)?;
        Ok(self.client.request_text(req).await?)
    }

    /// Fetch logs as a stream of bytes
    pub async fn log_stream(&self, name: &str, lp: &LogParams) -> Result<impl Stream<Item = Result<Bytes>>> {
        let req = self.resource.logs(name, lp)?;
        Ok(self.client.request_text_stream(req).await?)
    }
}
