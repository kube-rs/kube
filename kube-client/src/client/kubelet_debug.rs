use crate::{
    api::{AttachParams, AttachedProcess, LogParams, Portforwarder},
    client::AsyncBufRead,
    Client, Error, Result,
};
use kube_core::{node_proxy::KubeletDebugParams, Request};
use std::fmt::Debug;

/// Methods to access debug endpoints directly on `kubelet`
///
/// These are analogous to the `Pod` api methods for [`Execute`], [`Attach`], and [`Portforward`].
/// Service account must have `nodes/proxy` access, and
/// the debug handlers must be enabled either via `--enable-debugging-handlers ` or in the [kubelet config](https://kubernetes.io/docs/reference/config-api/kubelet-config.v1beta1/#kubelet-config-k8s-io-v1beta1-KubeletConfiguration).
/// See the [kubelet source](https://github.com/kubernetes/kubernetes/blob/b3926d137cd2964cd3a04088ded30845910547b1/pkg/kubelet/server/server.go#L454), and [kubelet reference](https://kubernetes.io/docs/reference/command-line-tools-reference/kubelet/) for more info.
///
/// ## Warning
/// These methods require direct and insecure access to `kubelet` and is only available under the `kubelet_debug` feature.
/// End-to-end usage is explored in the [pod_log_node_proxy](./examples/pod_log_node_proxy.rs) example.
#[cfg(feature = "kubelet_debug")]
impl Client {
    /// Attach to pod directly from the node
    ///
    /// ## Warning
    /// This method uses the insecure `kubelet_debug` interface.
    pub async fn node_attach(
        &self,
        node_proxy_params: &KubeletDebugParams<'_>,
        container: &str,
    ) -> Result<AttachedProcess> {
        let mut req = Request::node_attach(node_proxy_params, container, ap).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_attach");
        let stream = self.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }

    /// Execute a command in a pod directly from the node
    ///
    /// ## Warning
    /// This method uses the insecure `kubelet_debug` interface.
    pub async fn node_exec<I, T>(
        &self,
        node_proxy_params: &KubeletDebugParams<'_>,
        container: &str,
        command: I,
        ap: &AttachParams,
    ) -> Result<AttachedProcess>
    where
        I: IntoIterator<Item = T> + Debug,
        T: Into<String>,
    {
        let mut req =
            Request::node_exec(node_proxy_params, container, command, ap).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_exec");
        let stream = self.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }

    /// Forward ports of a pod directly from the node
    ///
    /// ## Warning
    /// This method uses the insecure `kubelet_debug` interface.
    pub async fn node_portforward(
        &self,
        node_proxy_params: &KubeletDebugParams<'_>,
        ports: &[u16],
    ) -> Result<Portforwarder> {
        let mut req = Request::node_portforward(node_proxy_params, ports).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_portforward");
        let stream = self.connect(req).await?;
        Ok(Portforwarder::new(stream, ports))
    }

    /// Stream logs directly from node
    ///
    /// ## Warning
    /// This method uses the insecure `kubelet_debug` interface.
    pub async fn node_logs(
        &self,
        node_proxy_params: &KubeletDebugParams<'_>,
        container: &str,
        lp: &LogParams,
    ) -> Result<impl AsyncBufRead> {
        let mut req = Request::node_logs(node_proxy_params, container, lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_log");
        self.request_stream(req).await
    }
}
