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
        //
        // When sending events to a client, Kubernetes may send 1 event per HTTP flush, it may send
        // 2, it may send 1.5, so we need to handle partial items per chunk.
        // What this unfold does:
        // Takes a mutable reqwest::Respose resp, a mutable Vec<u8> buff, and a mutable Vec<Result<T>> items
        // At each function call we will:
        // Check if items is not empty, if so pop the first element, yield it, and recurse
        // If items is empty, we will await the next chunk of resp and add it to buff. If the current
        // chunk contains a newline character, we will iterate over all the lines in buff,
        // adding any parsed (successfully or not) values to items, and any left over partial lines to a new buffer,
        // and recurse with these elements.
        //
        // Any reqwest errors will terminate this stream early.
        Ok(futures::stream::unfold(
            (res, Vec::new(), Vec::new()),
            |(mut resp, mut buff, mut items): (_, _, Vec<Result<T>>)| {
                async {
                    // If we have any items, pop off the first,
                    // yield it, and then continue into the next iteration
                    if !items.is_empty() {
                        let current = items.pop().expect("items should not be empty"); // We know items is not empty so this is safe
                        return Some((current, (resp, buff, items)));
                    }

                    // Loop pulling in chunks of the response until we have
                    // actionable values (something containing a newline)
                    loop {
                        match resp.chunk().await {
                            Ok(Some(chunk)) => {
                                // append it to our current buffer
                                buff.extend_from_slice(&chunk);

                                // If it contains a new line, we have a full item somewhere in our buffer
                                // Note: It seems that the kube api escapes all new line characters excepting the ones that
                                // separate distinct events.
                                if chunk.contains(&b'\n') {
                                    // Create a new buffer for our next iteration
                                    // to move any partial lines to
                                    // We an safely reuse the items vec as we know it's empty here.
                                    let mut new_buff = Vec::new();

                                    for line in buff.split(|x| x == &b'\n') {
                                        match serde_json::from_slice(&line) {
                                            Ok(val) => items.push(Ok(val)),
                                            Err(e) if e.is_eof() => {
                                                // If this is an EOF error then we do not have a complete
                                                // kube object, so push the current line into our new buffer
                                                // and add the newline character we stripped as part of the iterator.
                                                new_buff.extend_from_slice(&line);
                                                new_buff.push(b'\n');
                                            }
                                            Err(e) => items.push(Err(Error::SerdeError(e))),
                                        }
                                    }

                                    // This expect _should_ be safe, but if we panic this would be the first
                                    // place I'd look.
                                    // We could alternatively return some sort of error here if items is empty
                                    // so that we continue iterating, rather than returning None.
                                    let head = items.pop().expect("items should not be empty");
                                    return Some((head, (resp, new_buff, items)));
                                }
                            }
                            Ok(None) => return None,
                            Err(e) => {
                                return Some((Err(Error::ReqwestError(e)), (resp, buff, items)))
                            }
                        }
                    }
                }
            },
        ))
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
