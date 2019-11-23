//! A basic API client with standard kube error handling

use crate::config::Configuration;
use crate::{Error, ErrorResponse, Result};
use either::Either;
use either::{Left, Right};
use futures::{self, Stream};
use http;
use http::StatusCode;
use serde::de::DeserializeOwned;
use serde_json;
use serde_json::Value;

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
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
    pub retryAfterSeconds: u32,
}

#[derive(Deserialize, Debug)]
pub struct StatusCause {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub field: String,
}

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

/// APIClient requires `config::Configuration` includes client to connect with kubernetes cluster.
#[derive(Clone)]
pub struct APIClient {
    configuration: Configuration,
}

impl APIClient {
    pub fn new(configuration: Configuration) -> Self {
        APIClient { configuration }
    }

    async fn send(&self, request: http::Request<Vec<u8>>) -> Result<reqwest::Response> {
        let (parts, body) = request.into_parts();
        let uri_str = format!("{}{}", self.configuration.base_path, parts.uri);
        trace!("{} {}", parts.method, uri_str);
        //trace!("Request body: {:?}", String::from_utf8_lossy(&body));
        let req = match parts.method {
            http::Method::GET => self.configuration.client.get(&uri_str),
            http::Method::POST => self.configuration.client.post(&uri_str),
            http::Method::DELETE => self.configuration.client.delete(&uri_str),
            http::Method::PUT => self.configuration.client.put(&uri_str),
            http::Method::PATCH => self.configuration.client.patch(&uri_str),
            other => Err(Error::InvalidMethod(other.to_string()))?,
        }
        .headers(parts.headers)
        .body(body)
        .build()?;
        //trace!("Request Headers: {:?}", req.headers());
        let res = self.configuration.client.execute(req).await?;
        Ok(res)
    }

    pub async fn request<T>(&self, request: http::Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, &s)?;

        serde_json::from_str(&text).map_err(|e| {
            warn!("{}, {:?}", text, e);
            Error::SerdeError(e)
        })
    }

    pub async fn request_text(&self, request: http::Request<Vec<u8>>) -> Result<String> {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, &s)?;

        Ok(text)
    }

    pub async fn request_status<T>(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<Either<T, Status>>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, &s)?;

        // It needs to be JSON:
        let v: Value = serde_json::from_str(&text)?;
        if v["kind"] == "Status" {
            trace!("Status from {}", text);
            Ok(Right(serde_json::from_str::<Status>(&text).map_err(
                |e| {
                    warn!("{}, {:?}", text, e);
                    Error::SerdeError(e)
                },
            )?))
        } else {
            Ok(Left(serde_json::from_str::<T>(&text).map_err(|e| {
                warn!("{}, {:?}", text, e);
                Error::SerdeError(e)
            })?))
        }
    }

    pub async fn request_events<T>(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<impl Stream<Item = Result<T>>>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());

        // Now use `unfold` to convert the chunked responses into a Stream
        Ok(futures::stream::unfold(res, |mut resp| {
            async move {
                // A buffer to store our current object in
                // as we stream it's entirety
                let mut buff = Vec::new();

                loop {
                    match resp.chunk().await {
                        Ok(Some(chunk)) => {
                            // append it to our current buffer
                            buff.extend_from_slice(&chunk);

                            // If it ends with a newline, then it should be a complete resource
                            // so we try to parse it.
                            // On success we yield this item
                            // otherwise we either continue looping or break and return an error
                            if buff.ends_with(b"\n") {
                                match serde_json::from_slice::<T>(&buff) {
                                    Ok(val) => return Some((Ok(val), resp)),
                                    Err(e) => {
                                        warn!("{} {:?}", String::from_utf8_lossy(&buff), e);
                                        return Some((Err(Error::SerdeError(e)), resp));
                                    }
                                }
                            }
                        }
                        Ok(None) => return None,
                        Err(e) => return Some((Err(Error::ReqwestError(e)), resp)),
                    }
                }
            }
        }))
    }
}

/// Kubernetes returned error handling
///
/// Either kube returned an explicit ApiError struct,
/// or it someohow returned something we couldn't parse as one.
///
/// In either case, present an ApiError upstream.
/// The latter is probably a bug if encountered.
fn handle_api_errors(text: &str, s: &StatusCode) -> Result<()> {
    if s.is_client_error() || s.is_server_error() {
        // Print better debug when things do fail
        //trace!("Parsing error: {}", text);
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
