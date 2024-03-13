use crate::{
    api::{AttachParams, AttachedProcess, LogParams, Portforwarder},
    client::AsyncBufRead,
    Client, Error, Result,
};
use kube_core::{node_proxy::NodeProxyParams, Request};
use std::fmt::Debug;

/// Those endpoints will give you access to debug endpoints directly on kubelet
/// Usage is the same as associated Pod api methods
/// Service account must have `nodes/proxy` resources right
/// `--enable-debugging-handlers ` must be set to true (default) in kubelet config
/// ```no_run
/// let mut config = kube::Config::new(format!("https://{node_ip}:10250").try_into().expect("uri"));
/// config.accept_invalid_certs = true;
/// config.auth_info.token = Some(token) // Service account token
/// let client = kube::Client::try_from(config)?;
/// let logs = client
/// .node_logs(
///     &namespace,
///     &pod_name,
///     &container_name,
///     &LogParams {
///         tail_lines: logs_params.tail,
///         follow: logs_params.follow,
///         timestamps: true,
///         ..Default::default()
///     },
/// )
/// .await;
/// ```
///
impl Client {
    /// Attach to pod directly from the node
    pub async fn node_attach(
        &self,
        node_proxy_params: &NodeProxyParams,
        ap: &AttachParams,
    ) -> Result<AttachedProcess> {
        let mut req = Request::node_attach(node_proxy_params, ap).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_attach");
        let stream = self.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }

    /// Execute a command in a pod directly from the node
    pub async fn node_exec<I, T>(
        &self,
        node_proxy_params: &NodeProxyParams,
        command: I,
        ap: &AttachParams,
    ) -> Result<AttachedProcess>
    where
        I: IntoIterator<Item = T> + Debug,
        T: Into<String>,
    {
        let mut req = Request::node_exec(node_proxy_params, command, ap).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_exec");
        let stream = self.connect(req).await?;
        Ok(AttachedProcess::new(stream, ap))
    }

    /// Forward ports of a pod directly from the node
    pub async fn node_portforward(
        &self,
        namespace: &str,
        name: &str,
        ports: &[u16],
    ) -> Result<Portforwarder> {
        let mut req = Request::node_portforward(namespace, name, ports).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_portforward");
        let stream = self.connect(req).await?;
        Ok(Portforwarder::new(stream, ports))
    }

    /// Stream logs directly from node
    pub async fn node_logs(
        &self,
        node_proxy_params: &NodeProxyParams,
        lp: &LogParams,
    ) -> Result<impl AsyncBufRead> {
        let mut req = Request::node_logs(node_proxy_params, lp).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("node_log");
        self.request_stream(req).await
    }
}
