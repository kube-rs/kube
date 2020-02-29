use bytes::Bytes;
use futures::Stream;
use serde::de::DeserializeOwned;

use crate::{
    api::{
        resource::Object,
        Api, PatchParams, PostParams, RawApi,
    },
    Error, Result,
};


// ----------------------------------------------------------------------------

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

// ----------------------------------------------------------------------------


/// Status subresource
///
/// https://kubernetes.io/docs/tasks/access-kubernetes-api/custom-resources/custom-resource-definitions/#status-subresource
impl<K> Api<K>
where
    K: Clone + DeserializeOwned,
{
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

// ----------------------------------------------------------------------------
// Log subresource
// ----------------------------------------------------------------------------

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


impl<K> RawApi<K> {
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

#[cfg(feature = "openapi")]
#[test]
fn log_path() {
    use crate::api::RawApi;
    use k8s_openapi::api::core::v1 as corev1;
    let r = RawApi::<corev1::Pod>::within("ns");
    let mut lp = LogParams::default();
    lp.container = Some("blah".into());
    let req = r.logs("foo", &lp).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods/foo/log?&container=blah");
}

/// Marker trait for objects that has logs
pub trait LoggingObject {}

#[cfg(feature = "openapi")]
impl LoggingObject for k8s_openapi::api::core::v1::Pod {}

impl<K> Api<K>
where
    K: Clone + DeserializeOwned + LoggingObject,
{
    pub async fn log(&self, name: &str, lp: &LogParams) -> Result<String> {
        let req = self.api.logs(name, lp)?;
        Ok(self.client.request_text(req).await?)
    }

    pub async fn log_stream(&self, name: &str, lp: &LogParams) -> Result<impl Stream<Item = Result<Bytes>>> {
        let req = self.api.logs(name, lp)?;
        Ok(self.client.request_text_stream(req).await?)
    }
}
