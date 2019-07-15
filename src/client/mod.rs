//! A basic API client with standard kube error handling

use serde_json::Value;
use either::{Right, Left};
use either::Either;
use http::StatusCode;
use http;
use serde::de::DeserializeOwned;
use serde_json;
use failure::ResultExt;
use crate::{ApiError, Error, ErrorKind, Result};
use crate::config::Configuration;


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
    pub retryAfterSeconds: u32
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

    fn send(&self, request: http::Request<Vec<u8>>) -> Result<reqwest::Response>
    {
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
            other => Err(ErrorKind::InvalidMethod(other.to_string()))?
        }.headers(parts.headers).body(body).build().context(ErrorKind::RequestBuild)?;
        //trace!("Request Headers: {:?}", req.headers());
        Ok(self.configuration.client.execute(req).context(ErrorKind::RequestSend)?)
    }


    pub fn request<T>(&self, request: http::Request<Vec<u8>>) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut res : reqwest::Response = self.send(request)?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().context(ErrorKind::RequestParse)?;
        res.error_for_status().map_err(|e| make_api_error(&text, e, &s))?;

        serde_json::from_str(&text).map_err(|e| {
            warn!("{}, {:?}", text, e);
            Error::from(ErrorKind::SerdeParse)
        })
    }

    pub fn request_text(&self, request: http::Request<Vec<u8>>) -> Result<String>
    {
        let mut res : reqwest::Response = self.send(request)?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().context(ErrorKind::RequestParse)?;
        res.error_for_status().map_err(|e| make_api_error(&text, e, &s))?;

        Ok(text)
    }

    pub fn request_status<T>(&self, request: http::Request<Vec<u8>>) -> Result<Either<T, Status>>
    where
        T: DeserializeOwned,
    {
        let mut res : reqwest::Response = self.send(request)?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().context(ErrorKind::RequestParse)?;
        res.error_for_status().map_err(|e| make_api_error(&text, e, &s))?;

        // It needs to be JSON:
        let v: Value = serde_json::from_str(&text).context(ErrorKind::SerdeParse)?;;
        if v["kind"] == "Status" {
            trace!("Status from {}", text);
            Ok(Right(serde_json::from_str::<Status>(&text).map_err(|e| {
                warn!("{}, {:?}", text, e);
                Error::from(ErrorKind::SerdeParse)
            })?))
        } else {
            Ok(Left(serde_json::from_str::<T>(&text).map_err(|e| {
                warn!("{}, {:?}", text, e);
                Error::from(ErrorKind::SerdeParse)
            })?))
        }
    }

    pub fn request_events<T>(&self, request: http::Request<Vec<u8>>) -> Result<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let mut res : reqwest::Response = self.send(request)?;
        trace!("{} {}", res.status().as_str(), res.url());
        //trace!("Response Headers: {:?}", res.headers());
        let s = res.status();
        let text = res.text().context(ErrorKind::RequestParse)?;
        res.error_for_status().map_err(|e| make_api_error(&text, e, &s))?;

        // Should be able to coerce result into Vec<T> at this point
        let mut xs : Vec<T> = vec![];
        for l in text.lines() {
            let r = serde_json::from_str(&l).map_err(|e| {
                warn!("{} {:?}", l, e);
                Error::from(ErrorKind::SerdeParse)
            })?;
            xs.push(r);
        }
        Ok(xs)
    }
}

/// Kubernetes returned error handling
///
/// Either kube returned an explicit ApiError struct,
/// or it someohow returned something we couldn't parse as one.
///
/// In either case, present an ApiError upstream.
/// The latter is probably a bug if encountered.
fn make_api_error(text: &str, error: reqwest::Error, s: &StatusCode) -> ErrorKind {
    // Print better debug when things do fail
    //trace!("Parsing error: {}", text);
    if let Ok(errdata) = serde_json::from_str::<ApiError>(text) {
        debug!("Unsuccessful: {:?}", errdata);
        ErrorKind::Api(errdata)
    } else {
        warn!("Unsuccessful data error parse: {}", text);
        // Propagate errors properly via reqwest
        let ae = ApiError {
            status: s.to_string(),
            code: s.as_u16(),
            message: format!("{:?}", error),
            reason: format!("{}", error),
        };
        debug!("Unsuccessful: {:?} (reconstruct)", ae);
        ErrorKind::Api(ae)
    }
}
