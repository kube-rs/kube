//! A basic API client with standard kube error handling

use crate::{config::Configuration, Error, ErrorResponse, Result};
use bytes::Bytes;
use either::{Either, Left, Right};
use futures::{self, Stream, StreamExt};
use futures_util::TryStreamExt;
use http::{self, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{self, Value};

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
        // trace!("Request body: {:?}", String::from_utf8_lossy(&body));
        let req = match parts.method {
            http::Method::GET => self.configuration.client.get(&uri_str),
            http::Method::POST => self.configuration.client.post(&uri_str),
            http::Method::DELETE => self.configuration.client.delete(&uri_str),
            http::Method::PUT => self.configuration.client.put(&uri_str),
            http::Method::PATCH => self.configuration.client.patch(&uri_str),
            other => return Err(Error::InvalidMethod(other.to_string())),
        }
        .headers(parts.headers)
        .body(body)
        .build()?;
        // trace!("Request Headers: {:?}", req.headers());
        let res = self.configuration.client.execute(req).await?;
        Ok(res)
    }

    pub async fn request<T>(&self, request: http::Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        // trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, s)?;

        serde_json::from_str(&text).map_err(|e| {
            warn!("{}, {:?}", text, e);
            Error::SerdeError(e)
        })
    }

    pub async fn request_text(&self, request: http::Request<Vec<u8>>) -> Result<String> {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        // trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().await?;
        handle_api_errors(&text, s)?;

        Ok(text)
    }

    pub async fn request_text_stream(
        &self,
        request: http::Request<Vec<u8>>,
    ) -> Result<impl Stream<Item = Result<Bytes>>> {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());

        Ok(res.bytes_stream().map_err(Error::ReqwestError))
    }

    pub async fn request_status<T>(&self, request: http::Request<Vec<u8>>) -> Result<Either<T, Status>>
    where
        T: DeserializeOwned,
    {
        let res: reqwest::Response = self.send(request).await?;
        trace!("{} {}", res.status().as_str(), res.url());
        // trace!("Response Headers: {:?}", res.headers());
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
        // We first construct a Stream of Vec<Result<T>> as we potentially might need to
        // yield multiple objects per loop, then we flatten it to the Stream<Result<T>> as expected.
        // Any reqwest errors will terminate this stream early.
        let stream = futures::stream::unfold((res, Vec::new()), |(mut resp, mut buff)| {
            async {
                loop {
                    match resp.chunk().await {
                        Ok(Some(chunk)) => {
                            buff.extend_from_slice(&chunk);

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
                                                warn!("Failed to parse: {}", String::from_utf8_lossy(line));
                                                // Clear the buffer as this was a valid object
                                                new_buff.clear();
                                                items.push(Err(Error::SerdeError(e)));
                                            }
                                        }
                                    }
                                }

                                // Now return our items and loop
                                return Some((items, (resp, new_buff)));
                            }
                        }
                        Ok(None) => return None,
                        Err(e) => {
                            let err = vec![Err(Error::ReqwestError(e))];
                            return Some((err, (resp, buff)));
                        }
                    }
                }
            }
        });

        Ok(stream.map(futures::stream::iter).flatten())
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
