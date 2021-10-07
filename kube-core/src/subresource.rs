//! Request builder types and parameters for subresources
use std::fmt::Debug;

use crate::{
    params::{DeleteParams, PostParams},
    request::{Request, JSON_MIME},
    Error, Result,
};

pub use k8s_openapi::api::autoscaling::v1::{Scale, ScaleSpec, ScaleStatus};

// ----------------------------------------------------------------------------
// Log subresource
// ----------------------------------------------------------------------------

/// Params for logging
#[derive(Default, Clone, Debug)]
pub struct LogParams {
    /// The container for which to stream logs. Defaults to only container if there is one container in the pod.
    pub container: Option<String>,
    /// Follow the log stream of the pod. Defaults to `false`.
    pub follow: bool,
    /// If set, the number of bytes to read from the server before terminating the log output.
    /// This may not display a complete final line of logging, and may return slightly more or slightly less than the specified limit.
    pub limit_bytes: Option<i64>,
    /// If `true`, then the output is pretty printed.
    pub pretty: bool,
    /// Return previous terminated container logs. Defaults to `false`.
    pub previous: bool,
    /// A relative time in seconds before the current time from which to show logs.
    /// If this value precedes the time a pod was started, only logs since the pod start will be returned.
    /// If this value is in the future, no logs will be returned. Only one of sinceSeconds or sinceTime may be specified.
    pub since_seconds: Option<i64>,
    /// If set, the number of lines from the end of the logs to show.
    /// If not specified, logs are shown from the creation of the container or sinceSeconds or sinceTime
    pub tail_lines: Option<i64>,
    /// If `true`, add an RFC3339 or RFC3339Nano timestamp at the beginning of every line of log output. Defaults to `false`.
    pub timestamps: bool,
}

impl Request {
    /// Get a pod logs
    pub fn logs(&self, name: &str, lp: &LogParams) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}/log?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);

        if let Some(container) = &lp.container {
            qp.append_pair("container", container);
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

// ----------------------------------------------------------------------------
// Eviction subresource
// ----------------------------------------------------------------------------

/// Params for evictable objects
#[derive(Default, Clone)]
pub struct EvictParams {
    /// How the eviction should occur
    pub delete_options: Option<DeleteParams>,
    /// How the http post should occur
    pub post_options: PostParams,
}

impl Request {
    /// Create an eviction
    pub fn evict(&self, name: &str, ep: &EvictParams) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}/eviction?", self.url_path, name);
        // This is technically identical to Request::create, but different url
        let pp = &ep.post_options;
        pp.validate()?;
        let mut qp = form_urlencoded::Serializer::new(target);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        // eviction body parameters are awkward, need metadata with name
        let data = serde_json::to_vec(&serde_json::json!({
            "delete_options": ep.delete_options,
            "metadata": { "name": name }
        }))?;
        let req = http::Request::post(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::HttpError)
    }
}

// ----------------------------------------------------------------------------
// Attach subresource
// ----------------------------------------------------------------------------
/// Parameters for attaching to a container in a Pod.
///
/// - One of `stdin`, `stdout`, or `stderr` must be `true`.
/// - `stderr` and `tty` cannot both be `true` because multiplexing is not supported with TTY.
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
#[derive(Debug)]
pub struct AttachParams {
    /// The name of the container to attach.
    /// Defaults to the only container if there is only one container in the pod.
    pub container: Option<String>,
    /// Attach to the container's standard input. Defaults to `false`.
    ///
    /// Call [`AttachedProcess::stdin`](https://docs.rs/kube/*/kube/api/struct.AttachedProcess.html#method.stdin) to obtain a writer.
    pub stdin: bool,
    /// Attach to the container's standard output. Defaults to `true`.
    ///
    /// Call [`AttachedProcess::stdout`](https://docs.rs/kube/*/kube/api/struct.AttachedProcess.html#method.stdout) to obtain a reader.
    pub stdout: bool,
    /// Attach to the container's standard error. Defaults to `true`.
    ///
    /// Call [`AttachedProcess::stderr`](https://docs.rs/kube/*/kube/api/struct.AttachedProcess.html#method.stderr) to obtain a reader.
    pub stderr: bool,
    /// Allocate TTY. Defaults to `false`.
    ///
    /// NOTE: Terminal resizing is not implemented yet.
    pub tty: bool,

    /// The maximum amount of bytes that can be written to the internal `stdin`
    /// pipe before the write returns `Poll::Pending`.
    /// Defaults to 1024.
    ///
    /// This is not sent to the server.
    pub max_stdin_buf_size: Option<usize>,
    /// The maximum amount of bytes that can be written to the internal `stdout`
    /// pipe before the write returns `Poll::Pending`.
    /// Defaults to 1024.
    ///
    /// This is not sent to the server.
    pub max_stdout_buf_size: Option<usize>,
    /// The maximum amount of bytes that can be written to the internal `stderr`
    /// pipe before the write returns `Poll::Pending`.
    /// Defaults to 1024.
    ///
    /// This is not sent to the server.
    pub max_stderr_buf_size: Option<usize>,
}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Default for AttachParams {
    // Default matching the server's defaults.
    fn default() -> Self {
        Self {
            container: None,
            stdin: false,
            stdout: true,
            stderr: true,
            tty: false,
            max_stdin_buf_size: None,
            max_stdout_buf_size: None,
            max_stderr_buf_size: None,
        }
    }
}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl AttachParams {
    /// Default parameters for an tty exec with stdin and stdout
    pub fn interactive_tty() -> Self {
        Self {
            stdin: true,
            stdout: true,
            stderr: false,
            tty: true,
            ..Default::default()
        }
    }

    /// Specify the container to execute in.
    pub fn container<T: Into<String>>(mut self, container: T) -> Self {
        self.container = Some(container.into());
        self
    }

    /// Set `stdin` field.
    pub fn stdin(mut self, enable: bool) -> Self {
        self.stdin = enable;
        self
    }

    /// Set `stdout` field.
    pub fn stdout(mut self, enable: bool) -> Self {
        self.stdout = enable;
        self
    }

    /// Set `stderr` field.
    pub fn stderr(mut self, enable: bool) -> Self {
        self.stderr = enable;
        self
    }

    /// Set `tty` field.
    pub fn tty(mut self, enable: bool) -> Self {
        self.tty = enable;
        self
    }

    /// Set `max_stdin_buf_size` field.
    pub fn max_stdin_buf_size(mut self, size: usize) -> Self {
        self.max_stdin_buf_size = Some(size);
        self
    }

    /// Set `max_stdout_buf_size` field.
    pub fn max_stdout_buf_size(mut self, size: usize) -> Self {
        self.max_stdout_buf_size = Some(size);
        self
    }

    /// Set `max_stderr_buf_size` field.
    pub fn max_stderr_buf_size(mut self, size: usize) -> Self {
        self.max_stderr_buf_size = Some(size);
        self
    }

    fn validate(&self) -> Result<()> {
        if !self.stdin && !self.stdout && !self.stderr {
            return Err(Error::RequestValidation(
                "AttachParams: one of stdin, stdout, or stderr must be true".into(),
            ));
        }

        if self.stderr && self.tty {
            // Multiplexing is not supported with TTY
            return Err(Error::RequestValidation(
                "AttachParams: tty and stderr cannot both be true".into(),
            ));
        }

        Ok(())
    }

    fn append_to_url_serializer(&self, qp: &mut form_urlencoded::Serializer<String>) {
        if self.stdin {
            qp.append_pair("stdin", "true");
        }
        if self.stdout {
            qp.append_pair("stdout", "true");
        }
        if self.stderr {
            qp.append_pair("stderr", "true");
        }
        if self.tty {
            qp.append_pair("tty", "true");
        }
        if let Some(container) = &self.container {
            qp.append_pair("container", container);
        }
    }
}

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Request {
    /// Attach to a pod
    pub fn attach(&self, name: &str, ap: &AttachParams) -> Result<http::Request<Vec<u8>>> {
        ap.validate()?;

        let target = format!("{}/{}/attach?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        ap.append_to_url_serializer(&mut qp);

        let req = http::Request::get(qp.finish());
        req.body(vec![]).map_err(Error::HttpError)
    }
}

// ----------------------------------------------------------------------------
// Exec subresource
// ----------------------------------------------------------------------------
#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
impl Request {
    /// Execute command in a pod
    pub fn exec<I, T>(&self, name: &str, command: I, ap: &AttachParams) -> Result<http::Request<Vec<u8>>>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        ap.validate()?;

        let target = format!("{}/{}/exec?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        ap.append_to_url_serializer(&mut qp);

        for c in command.into_iter() {
            qp.append_pair("command", &c.into());
        }

        let req = http::Request::get(qp.finish());
        req.body(vec![]).map_err(Error::HttpError)
    }
}
