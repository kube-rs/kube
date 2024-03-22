//! Node proxy methods
use crate::{
    request::Error,
    subresource::{AttachParams, LogParams},
    Request,
};
use std::fmt::Debug;

/// Struct that hold all required parameters to call specific pod methods from node
#[derive(Default)]
pub struct KubeletDebugParams<'a> {
    /// Name of the pod
    pub name: &'a str,
    /// Namespace of the pod
    pub namespace: &'a str,
    /// Pod uid used to ensure that the pod name matches the pod uid
    pub pod_uid: Option<&'a str>,
}

impl KubeletDebugParams<'_> {
    fn with_uid(&self) -> String {
        if let Some(uid) = &self.pod_uid {
            format!("{}/{}/{}", self.namespace, self.name, uid)
        } else {
            self.without_uid()
        }
    }

    fn without_uid(&self) -> String {
        format!("{}/{}", self.namespace, self.name)
    }
}

impl Request {
    /// Attach to pod directly from the node
    pub fn kubelet_node_attach(
        kubelet_debug_params: &KubeletDebugParams<'_>,
        container: &str,
        ap: &AttachParams,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        ap.validate()?;

        let target = format!("/attach/{}/{container}?", kubelet_debug_params.with_uid());
        let mut qp = form_urlencoded::Serializer::new(target);
        ap.append_to_url_serializer_local(&mut qp);

        let req = http::Request::get(qp.finish());
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Execute a command in a pod directly from the node
    pub fn kubelet_node_exec<I, T>(
        kubelet_debug_params: &KubeletDebugParams<'_>,
        container: &str,
        command: I,
        ap: &AttachParams,
    ) -> Result<http::Request<Vec<u8>>, Error>
    where
        I: IntoIterator<Item = T> + Debug,
        T: Into<String>,
    {
        ap.validate()?;

        let target = format!("/exec/{}/{container}?", kubelet_debug_params.with_uid());
        let mut qp = form_urlencoded::Serializer::new(target);
        ap.append_to_url_serializer_local(&mut qp);

        for c in command.into_iter() {
            qp.append_pair("command", &c.into());
        }

        let req = http::Request::get(qp.finish());
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Forward ports of a pod directly from the node
    pub fn kubelet_node_portforward(
        kubelet_debug_params: &KubeletDebugParams<'_>,
        ports: &[u16],
    ) -> Result<http::Request<Vec<u8>>, Error> {
        if ports.is_empty() {
            return Err(Error::Validation("ports cannot be empty".into()));
        }
        if ports.len() > 128 {
            return Err(Error::Validation(
                "the number of ports cannot be more than 128".into(),
            ));
        }

        if ports.len() > 1 {
            let mut seen = std::collections::HashSet::with_capacity(ports.len());
            for port in ports.iter() {
                if seen.contains(port) {
                    return Err(Error::Validation(format!(
                        "ports must be unique, found multiple {port}"
                    )));
                }
                seen.insert(port);
            }
        }

        let base_url = format!("/portForward/{}?", kubelet_debug_params.with_uid());
        let mut qp = form_urlencoded::Serializer::new(base_url);
        qp.append_pair(
            "port",
            &ports.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(","),
        );
        let req = http::Request::get(qp.finish());
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Stream logs directly from node
    pub fn kubelet_node_logs(
        kubelet_debug_params: &KubeletDebugParams<'_>,
        container: &str,
        lp: &LogParams,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        // Node logs is the only one that doesn't accept an uid for pod
        let target = format!(
            "/containerLogs/{}/{container}?",
            kubelet_debug_params.without_uid()
        );

        let mut qp = form_urlencoded::Serializer::new(target);

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
        } else if let Some(st) = &lp.since_time {
            let ser_since = st.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            qp.append_pair("sinceTime", &ser_since);
        }

        if let Some(tl) = &lp.tail_lines {
            qp.append_pair("tailLines", &tl.to_string());
        }

        if lp.timestamps {
            qp.append_pair("timestamps", "true");
        }

        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::BuildRequest)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        kubelet_debug::KubeletDebugParams,
        subresource::{AttachParams, LogParams},
        Request,
    };
    #[test]
    fn node_attach_test() {
        let req = Request::kubelet_node_attach(
            &KubeletDebugParams {
                name: "some-name",
                namespace: "some-namespace",
                pod_uid: Some("some-uid"),
            },
            "some-container",
            &AttachParams::default().stdin(true).stderr(true).stdout(true),
        )
        .unwrap();
        assert_eq!(
            req.uri(),
            "/attach/some-namespace/some-name/some-uid/some-container?&input=1&output=1&error=1"
        );
    }

    #[test]
    fn node_exec_test() {
        let req = Request::kubelet_node_exec(
            &KubeletDebugParams {
                name: "some-name",
                namespace: "some-namespace",
                pod_uid: None,
            },
            "some-container",
            "ls -l".split_whitespace(),
            &AttachParams::interactive_tty(),
        )
        .unwrap();
        assert_eq!(
            req.uri(),
            "/exec/some-namespace/some-name/some-container?&input=1&output=1&tty=1&command=ls&command=-l"
        );
    }

    #[test]
    fn node_logs_test() {
        let lp = LogParams {
            tail_lines: Some(10),
            follow: true,
            timestamps: true,
            ..Default::default()
        };
        let req = Request::kubelet_node_logs(
            &KubeletDebugParams {
                name: "some-name",
                namespace: "some-namespace",
                pod_uid: None,
            },
            "some-container",
            &lp,
        )
        .unwrap();
        assert_eq!(
            req.uri(),
            "/containerLogs/some-namespace/some-name/some-container?&follow=true&tailLines=10&timestamps=true"
        );
    }

    #[test]
    fn node_portforward_test() {
        let req = Request::kubelet_node_portforward(
            &KubeletDebugParams {
                name: "some-name",
                namespace: "some-namespace",
                pod_uid: None,
            },
            &[1204],
        )
        .unwrap();
        assert_eq!(req.uri(), "/portForward/some-namespace/some-name?&port=1204");
    }
}
