//! A basic API client with standard kube error handling

use http;
pub use http::StatusCode;
use serde::de::DeserializeOwned;
use serde_json;
use failure::ResultExt;
use crate::{ApiError, Error, ErrorKind, Result};
use crate::config::Configuration;

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
        match res.error_for_status() {
            Err(e) => {
                // Print better debug when things do fail
                if let Ok(errdata) = serde_json::from_str::<ApiError>(&text) {
                    debug!("Unsuccessful: {:?}", errdata);
                    Err(ErrorKind::Api(errdata))?;
                } else {
                    // In case some parts of ApiError for some reason don't exist..
                    error!("Unsuccessful data error parse: {}", text);
                    Err(ErrorKind::SerdeParse)?; // should not happen
                }
                // Propagate errors properly via reqwest
                let ae = ApiError {
                    status: s.to_string(),
                    code: s.as_u16(),
                    message: format!("{:?}", e),
                    reason: format!("{}", e),
                };
                debug!("Unsuccessful: {:?} (reconstruct)", ae);
                Err(ErrorKind::Api(ae))?
            },
            Ok(_res) => {
                serde_json::from_str(&text).map_err(|e| {
                    warn!("{}, {:?}", text, e);
                    Error::from(ErrorKind::SerdeParse)
                })
            }
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
        match res.error_for_status() {
            Err(e) => {
                // Print better debug when things do fail
                if let Ok(errdata) = serde_json::from_str::<ApiError>(&text) {
                    debug!("Unsuccessful: {:?}", errdata);
                    Err(ErrorKind::Api(errdata))?;
                } else {
                    // In case some parts of ApiError for some reason don't exist..
                    error!("Unsuccessful data error parse: {}", text);
                    Err(ErrorKind::SerdeParse)?; // should not happen
                }
                // Propagate errors properly via reqwest
                let ae = ApiError {
                    status: s.to_string(),
                    code: s.as_u16(),
                    message: format!("{:?}", e),
                    reason: format!("{}", e),
                };
                debug!("Unsuccessful: {:?} (reconstruct)", ae);
                Err(ErrorKind::Api(ae))?
            },
            Ok(_res) => {
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
            },
        }
    }
}
