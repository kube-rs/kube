//! A basic API client for interacting with the Kubernetes API
//!
//! The [`Client`] uses standard kube error handling.
//!
//! This client can be used on its own or in conjuction with
//! the [`Api`][crate::api::Api] type for more structured
//! interaction with the kuberneres API.

use crate::{
    api::{Meta, WatchEvent},
    config::Config,
    error::ErrorResponse,
    Error, Result,
};

#[cfg(feature = "ws")] mod ws;
#[cfg(feature = "ws")] use ws::AsyncTlsConnector;

#[cfg(feature = "ws")]
use async_tungstenite::{
    tokio::{connect_async_with_tls_connector, ConnectStream},
    tungstenite as ws2, WebSocketStream,
};
use bytes::Bytes;
use either::{Either, Left, Right};
use futures::{self, Stream, TryStream, TryStreamExt};
use http::{self, Request, StatusCode};
use k8s_openapi::apimachinery::pkg::apis::meta::v1 as k8s_meta_v1;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{self, Value};

use std::convert::{TryFrom, TryInto};

/// Client for connecting with a Kubernetes cluster.
///
/// The best way to instantiate the client is either by
/// inferring the configuration from the environment using
/// [`Client::try_default`] or with an existing [`Config`]
/// using [`Client::new`]
#[derive(Clone)]
pub struct Client {
    cluster_url: reqwest::Url,
    default_ns: String,
    inner: reqwest::Client,
    config: Config,
    #[cfg(feature = "ws")]
    tls_connector: AsyncTlsConnector,
}

impl Client {
    /// Create and initialize a [`Client`] using the given
    /// configuration.
    ///
    /// # Panics
    ///
    /// Panics if the configuration supplied leads to an invalid HTTP client.
    /// Refer to the [`reqwest::ClientBuilder::build`] docs for information
    /// on situations where this might fail. If you want to handle this error case
    /// use [`Config::try_from`](Self::try_from) (note that this requires [`std::convert::TryFrom`]
    /// to be in scope.)
    pub fn new(config: Config) -> Self {
        Self::try_from(config).expect("Could not create a client from the supplied config")
    }

    /// Create and initialize a [`Client`] using the inferred
    /// configuration.
    ///
    /// Will use [`Config::infer`] to try in-cluster enironment
    /// variables first, then fallback to the local kubeconfig.
    ///
    /// Will fail if neither configuration could be loaded.
    ///
    /// If you already have a [`Config`] then use [`Client::try_from`](Self::try_from)
    /// instead
    pub async fn try_default() -> Result<Self> {
        let client_config = Config::infer().await?;
        Self::try_from(client_config)
    }

    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<reqwest::Response> {
        let (parts, body) = request.into_parts();
        let pandq = parts.uri.path_and_query().expect("valid path+query from kube");
        let uri_str = finalize_url(&self.cluster_url, &pandq);
        //trace!("Sending request => method = {} uri = {}", parts.method, &uri_str);

        let mut headers = parts.headers;
        // If we have auth headers set, make sure they are updated and attached to the request
        if let Some(auth_header) = self.config.get_auth_header().await? {
            headers.insert(reqwest::header::AUTHORIZATION, auth_header);
        }

        let request = match parts.method {
            http::Method::GET
            | http::Method::POST
            | http::Method::DELETE
            | http::Method::PUT
            | http::Method::PATCH => self.inner.request(parts.method, &uri_str),
            other => return Err(Error::InvalidMethod(other.to_string())),
        };

        let req = request.headers(headers).body(body).build()?;
        let res = self.inner.execute(req).await?;
        Ok(res)
    }

    /// Make WebSocket connection.
    #[cfg(feature = "ws")]
    #[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
    pub async fn connect(&self, request: http::Request<()>) -> Result<WebSocketStream<ConnectStream>> {
        let (mut parts, _) = request.into_parts();
        if let Some(auth_header) = self.config.get_auth_header().await? {
            parts.headers.insert(http::header::AUTHORIZATION, auth_header);
        }
        // Use the binary subprotocol v4, to get JSON `Status` object in `error` channel (3).
        // There's no official documentation about this protocol, but it's described in
        // [`k8s.io/apiserver/pkg/util/wsstream/conn.go`](https://git.io/JLQED).
        // There's a comment about v4 and `Status` object in
        // [`kublet/cri/streaming/remotecommand/httpstream.go`](https://git.io/JLQEh).
        parts.headers.insert(
            "sec-websocket-protocol",
            "v4.channel.k8s.io".parse().expect("valid header value"),
        );
        // Replace scheme to ws(s).
        let pandq = parts.uri.path_and_query().expect("valid path+query from kube");
        parts.uri = finalize_url(&self.cluster_url, &pandq)
            .replacen("http", "ws", 1)
            .parse()
            .expect("valid URL");

        let tls = Some(self.tls_connector.clone());
        match connect_async_with_tls_connector(http::Request::from_parts(parts, ()), tls).await {
            Ok((stream, _)) => Ok(stream),

            Err(err) => match err {
                // tungstenite only gives us the status code.
                ws2::Error::Http(code) => Err(Error::Api(ErrorResponse {
                    status: code.to_string(),
                    code: code.as_u16(),
                    message: "".to_owned(),
                    reason: "".to_owned(),
                })),

                ws2::Error::HttpFormat(err) => Err(Error::HttpError(err)),

                // `tungstenite::Error::Tls` is only available when using `ws-native-tls` (`async-tungstenite/tokio-native-tls`)
                // because it comes from `tungstenite/tls` feature.
                #[cfg(feature = "ws-native-tls")]
                ws2::Error::Tls(err) => Err(Error::SslError(format!("{}", err))),

                // URL errors:
                // - No host found in URL
                // - Unsupported scheme (not ws/wss)
                // shouldn't happen in our case
                ws2::Error::Url(msg) => Err(Error::RequestValidation(msg.into())),

                // Protocol errors:
                // - Only GET is supported
                // - Only HTTP version >= 1.1 is supported
                // shouldn't happen in our case
                ws2::Error::Protocol(msg) => Err(Error::RequestValidation(msg.into())),

                ws2::Error::Io(err) => Err(Error::Connection(err)),

                // Unexpected errors. `tungstenite::Error` contains errors that doesn't happen when trying to conect.
                ws2::Error::ConnectionClosed
                | ws2::Error::AlreadyClosed
                | ws2::Error::Utf8
                | ws2::Error::Capacity(_)
                | ws2::Error::SendQueueFull(_) => Err(Error::WsOther(err.to_string())),
            },
        }
    }

    /// Perform a raw HTTP request against the API and deserialize the response
    /// as JSON to some known type.
    pub async fn request<T>(&self, request: http::Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let text = self.request_text(request).await?;

        serde_json::from_str(&text).map_err(|e| {
            warn!("{}, {:?}", text, e);
            Error::SerdeError(e)
        })
    }

    /// Perform a raw HTTP request against the API and get back the response
    /// as a string
    pub async fn request_text(&self, request: http::Request<Vec<u8>>) -> Result<String> {
        let res: reqwest::Response = self.send(request).await?;
        trace!("Status = {:?} for {}", res.status(), res.url());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, s)?;

        Ok(text)
    }

    /// Perform a raw HTTP request against the API and get back the response
    /// as a stream of bytes
    pub async fn request_text_stream(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<impl Stream<Item = Result<Bytes>>> {
        let res: reqwest::Response = self.send(request).await?;
        trace!("Status = {:?} for {}", res.status(), res.url());

        Ok(res.bytes_stream().map_err(Error::ReqwestError))
    }

    /// Perform a raw HTTP request against the API and get back either an object
    /// deserialized as JSON or a [`Status`] Object.
    pub async fn request_status<T>(&self, request: http::Request<Vec<u8>>) -> Result<Either<T, Status>>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("Status = {:?} for {}", res.status(), res.url());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, s)?;

        // It needs to be JSON:
        let v: Value = serde_json::from_str(&text)?;
        if v["kind"] == "Status" {
            trace!("Status from {}", text);
            Ok(Right(serde_json::from_str::<Status>(&text).map_err(|e| {
                warn!("{}, {:?}", text, e);
                Error::SerdeError(e)
            })?))
        } else {
            Ok(Left(serde_json::from_str::<T>(&text).map_err(|e| {
                warn!("{}, {:?}", text, e);
                Error::SerdeError(e)
            })?))
        }
    }

    /// Perform a raw request and get back a stream of [`WatchEvent`] objects
    pub async fn request_events<T: Clone + Meta>(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<impl TryStream<Item = Result<WatchEvent<T>>>>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("Streaming from {} -> {}", res.url(), res.status().as_str());
        trace!("headers: {:?}", res.headers());

        // Now unfold the chunked responses into a Stream
        // We first construct a Stream of Vec<Result<T>> as we potentially might need to
        // yield multiple objects per loop, then we flatten it to the Stream<Result<T>> as expected.
        // Any reqwest errors will terminate this stream early.

        let stream = futures::stream::try_unfold((res, Vec::new()), |(mut resp, _buff)| {
            async {
                let mut buff = _buff; // can be avoided, see #145
                loop {
                    trace!("Await chunk");
                    match resp.chunk().await {
                        Ok(Some(chunk)) => {
                            trace!("Some chunk of len {}", chunk.len());
                            //trace!("Chunk contents: {}", String::from_utf8_lossy(&chunk));
                            buff.extend_from_slice(&chunk);

                            //if buff.len() > 32000 { // TEST CASE
                            //    return Err(Error::InvalidMethod(format!("{}", buff.len())));
                            //}

                            // If we've encountered a newline, see if we have any items to yield
                            if chunk.contains(&b'\n') {
                                let mut new_buff = Vec::new();
                                let mut items = Vec::new();

                                // Split on newlines
                                for line in buff.split(|x| x == &b'\n') {
                                    new_buff.extend_from_slice(&line);

                                    match serde_json::from_slice(&new_buff) {
                                        Ok(val) => {
                                            // on success clear our buffer
                                            new_buff.clear();
                                            items.push(Ok(val));
                                        }
                                        Err(e) => {
                                            // If this is not an eof error it's a parse error
                                            // so log it and store it
                                            // Otherwise we don't do anything as we've already
                                            // added in the current partial line to our buffer for
                                            // use in the next loop
                                            if !e.is_eof() {
                                                // Check if it's a general API error response
                                                let e = match serde_json::from_slice(&new_buff) {
                                                    Ok(e) => Error::Api(e),
                                                    _ => {
                                                        let line = String::from_utf8_lossy(line);
                                                        warn!("Failed to parse: {}", line);
                                                        match resp.error_for_status_ref() {
                                                            Ok(_) => Error::SerdeError(e),
                                                            Err(e) => Error::ReqwestError(e),
                                                        }
                                                    }
                                                };

                                                // Clear the buffer as this was a valid object
                                                new_buff.clear();
                                                items.push(Err(e));
                                            }
                                        }
                                    }
                                }

                                // Now return our items and loop
                                return Ok(Some((items, (resp, new_buff))));
                            }
                        }
                        Ok(None) => {
                            trace!("None chunk");
                            return Ok(None);
                        }
                        Err(e) => {
                            if e.is_timeout() {
                                warn!("timeout in poll: {}", e); // our client timeout
                                return Ok(None);
                            }
                            let inner = e.to_string();
                            if inner.contains("unexpected EOF during chunk") {
                                // ^ catches reqwest::Error from hyper::Error
                                // where the inner.kind == UnexpectedEof
                                // and the inner.error == "unexpected EOF during chunk size line"
                                warn!("eof in poll: {}", e);
                                return Ok(None);
                            } else {
                                // There might be other errors worth ignoring here
                                // For now, if they happen, we hard error up
                                // This causes a full re-list for Reflector
                                error!("err poll: {:?} - {}", e, inner);
                                return Err(Error::ReqwestError(e));
                            }
                        }
                    }
                }
            }
        });

        Ok(stream.map_ok(futures::stream::iter).try_flatten())
    }

    /// Returns apiserver version.
    pub async fn apiserver_version(&self) -> Result<k8s_openapi::apimachinery::pkg::version::Info> {
        self.request(Request::builder().uri("/version").body(vec![])?)
            .await
    }

    /// Lists api groups that apiserver serves.
    pub async fn list_api_groups(&self) -> Result<k8s_meta_v1::APIGroupList> {
        self.request(Request::builder().uri("/apis").body(vec![])?).await
    }

    /// Lists resources served in given API group.
    ///
    /// ### Example usage:
    /// ```rust
    /// # async fn scope(client: kube::Client) -> Result<(), Box<dyn std::error::Error>> {
    /// let apigroups = client.list_api_groups().await?;
    /// for g in apigroups.groups {
    ///     let ver = g
    ///         .preferred_version
    ///         .as_ref()
    ///         .or_else(|| g.versions.first())
    ///         .expect("preferred or versions exists");
    ///     let apis = client.list_api_group_resources(&ver.group_version).await?;
    ///     dbg!(apis);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_api_group_resources(&self, apiversion: &str) -> Result<k8s_meta_v1::APIResourceList> {
        let url = format!("/apis/{}", apiversion);
        self.request(Request::builder().uri(url).body(vec![])?).await
    }

    /// Lists versions of `core` a.k.a. `""` legacy API group.
    pub async fn list_core_api_versions(&self) -> Result<k8s_meta_v1::APIVersions> {
        self.request(Request::builder().uri("/api").body(vec![])?).await
    }

    /// Lists resources served in particular `core` group version.
    pub async fn list_core_api_resources(&self, version: &str) -> Result<k8s_meta_v1::APIResourceList> {
        let url = format!("/api/{}", version);
        self.request(Request::builder().uri(url).body(vec![])?).await
    }
}

/// Kubernetes returned error handling
///
/// Either kube returned an explicit ApiError struct,
/// or it someohow returned something we couldn't parse as one.
///
/// In either case, present an ApiError upstream.
/// The latter is probably a bug if encountered.
fn handle_api_errors(text: &str, s: StatusCode) -> Result<()> {
    if s.is_client_error() || s.is_server_error() {
        // Print better debug when things do fail
        // trace!("Parsing error: {}", text);
        if let Ok(errdata) = serde_json::from_str::<ErrorResponse>(text) {
            debug!("Unsuccessful: {:?}", errdata);
            Err(Error::Api(errdata))
        } else {
            warn!("Unsuccessful data error parse: {}", text);
            // Propagate errors properly via reqwest
            let ae = ErrorResponse {
                status: s.to_string(),
                code: s.as_u16(),
                message: format!("{:?}", text),
                reason: "Failed to parse error data".into(),
            };
            debug!("Unsuccessful: {:?} (reconstruct)", ae);
            Err(Error::Api(ae))
        }
    } else {
        Ok(())
    }
}

impl TryFrom<Config> for Client {
    type Error = Error;

    /// Convert [`Config`] into a [`Client`]
    fn try_from(config: Config) -> Result<Self> {
        let cluster_url = config.cluster_url.clone();
        let default_ns = config.default_ns.clone();
        let config_clone = config.clone();
        #[cfg(feature = "ws")]
        let tls_connector: AsyncTlsConnector = config.clone().try_into()?;
        let builder: reqwest::ClientBuilder = config.try_into()?;
        Ok(Self {
            cluster_url,
            default_ns,
            inner: builder.build()?,
            #[cfg(feature = "ws")]
            tls_connector,
            config: config_clone,
        })
    }
}

impl TryFrom<Config> for reqwest::ClientBuilder {
    type Error = Error;

    fn try_from(config: Config) -> Result<Self> {
        let mut builder = Self::new();

        if let Some(i) = &config.proxy {
            builder = builder.proxy(i.clone())
        }

        if let Some(i) = config.identity() {
            builder = builder.identity(i)
        }

        if let Some(ders) = config.root_cert {
            for der in ders {
                builder = builder.add_root_certificate(der.try_into()?);
            }
        }

        if let Some(t) = config.timeout {
            builder = builder.timeout(t);
        }

        builder = builder.default_headers(config.headers);
        builder = builder.danger_accept_invalid_certs(config.accept_invalid_certs);

        Ok(builder)
    }
}

// TODO: replace with Status in k8s openapi?

/// A Kubernetes status object
#[allow(missing_docs)]
#[derive(Deserialize, Debug)]
pub struct Status {
    // TODO: typemeta
    // TODO: metadata that can be completely empty (listmeta...)
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<StatusDetails>,
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub code: u16,
}

/// Status details object on the [`Status`] object
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct StatusDetails {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub group: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub uid: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub causes: Vec<StatusCause>,
    #[serde(default, skip_serializing_if = "num::Zero::is_zero")]
    pub retry_after_seconds: u32,
}

/// Status cause object on the [`StatusDetails`] object
#[derive(Deserialize, Debug)]
#[allow(missing_docs)]
pub struct StatusCause {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
}

/// An internal url joiner to deal with the two different interfaces
///
/// - api module produces a http::Uri which we can turn into a PathAndQuery (has a leading slash by construction)
/// - config module produces a url::Url from user input (sometimes contains path segments)
///
/// This deals with that in a pretty easy way (tested below)
fn finalize_url(cluster_url: &reqwest::Url, request_pandq: &http::uri::PathAndQuery) -> String {
    let base = cluster_url.as_str().trim_end_matches('/'); // pandq always starts with a slash
    format!("{}{}", base, request_pandq)
}

#[cfg(test)]
mod test {
    use super::Status;

    // ensure our status schema is sensible
    #[test]
    fn delete_deserialize_test() {
        let statusresp = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"name":"some-app","group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s: Status = serde_json::from_str::<Status>(statusresp).unwrap();
        assert_eq!(s.details.unwrap().name, "some-app");

        let statusnoname = r#"{"kind":"Status","apiVersion":"v1","metadata":{},"status":"Success","details":{"group":"clux.dev","kind":"foos","uid":"1234-some-uid"}}"#;
        let s2: Status = serde_json::from_str::<Status>(statusnoname).unwrap();
        assert_eq!(s2.details.unwrap().name, ""); // optional probably better..
    }

    #[test]
    fn normal_host() {
        let minikube_host = "https://192.168.1.65:8443";
        let cluster_url = reqwest::Url::parse(minikube_host).unwrap();
        let apipath: http::Uri = "/api/v1/nodes?hi=yes".parse().unwrap();
        let pandq = apipath.path_and_query().expect("could pandq apipath");
        let final_url = super::finalize_url(&cluster_url, &pandq);
        assert_eq!(
            final_url.as_str(),
            "https://192.168.1.65:8443/api/v1/nodes?hi=yes"
        );
    }

    #[test]
    fn rancher_host() {
        // in rancher, kubernetes server names are not hostnames, but a host with a path:
        let rancher_host = "https://hostname/foo/bar";
        let cluster_url = reqwest::Url::parse(rancher_host).unwrap();
        assert_eq!(cluster_url.host_str().unwrap(), "hostname");
        assert_eq!(cluster_url.path(), "/foo/bar");
        // we must be careful when using Url::join on our http::Uri result
        // as a straight two Uri::join would trim away rancher's initial path
        // case in point (discards original path):
        assert_eq!(cluster_url.join("/api/v1/nodes").unwrap().path(), "/api/v1/nodes");

        let apipath: http::Uri = "/api/v1/nodes?hi=yes".parse().unwrap();
        let pandq = apipath.path_and_query().expect("could pandq apipath");

        let final_url = super::finalize_url(&cluster_url, &pandq);
        assert_eq!(final_url.as_str(), "https://hostname/foo/bar/api/v1/nodes?hi=yes");
    }
}
