//! A basic API client with standard kube error handling

use failure::Error;
use http;
use serde::de::DeserializeOwned;
use serde_json;

use crate::config::Configuration;

/// APIClient requires `config::Configuration` includes client to connect with kubernetes cluster.
#[derive(Clone)]
pub struct APIClient {
    configuration: Configuration,
}

/// Error data returned by kube
///
/// Replacement data for reqwest::Response::error_for_status
/// because it hardly ever includes good permission errors
#[derive(Deserialize, Debug)]
pub struct ApiError {
    status: String,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    reason: Option<String>,
    code: u16,
}

impl APIClient {
    pub fn new(configuration: Configuration) -> Self {
        APIClient { configuration }
    }

    fn send(&self, request: http::Request<Vec<u8>>) -> Result<reqwest::Response, Error>
    {
        let (parts, body) = request.into_parts();
        let uri_str = format!("{}{}", self.configuration.base_path, parts.uri);
        let req = match parts.method {
            http::Method::GET => self.configuration.client.get(&uri_str),
            http::Method::POST => self.configuration.client.post(&uri_str),
            http::Method::DELETE => self.configuration.client.delete(&uri_str),
            http::Method::PUT => self.configuration.client.put(&uri_str),
            other => {
                return Err(Error::from(format_err!("Invalid method: {}", other)));
            }
        }.body(body);
        trace!("{} {}", parts.method, uri_str);
        Ok(req.send()?)
    }


    pub fn request<T>(&self, request: http::Request<Vec<u8>>) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let mut res : reqwest::Response = self.send(request)?;
        if !res.status().is_success() {
            let text = res.text()?;
            // Print better debug when things do fail
            if let Ok(errdata) = serde_json::from_str::<ApiError>(&text) {
                warn!("Unsuccessful: {:?}", errdata);
            } else {
                // In case some parts of ApiError for some reason don't exist..
                warn!("Unsuccessful data: {}", text);
            }
            // Propagate errors properly via reqwest
            let e = res.error_for_status().unwrap_err();
            Err(e.into())
        } else {
            // Should be able to coerce result into T at this point
            let text = res.text()?;
            serde_json::from_str(&text).map_err(|e| {
                warn!("{}", text);
                Error::from(e)
            })
        }
    }

    pub fn request_events<T>(&self, request: http::Request<Vec<u8>>) -> Result<Vec<T>, Error>
    where
        T: DeserializeOwned,
    {
        let mut res : reqwest::Response = self.send(request)?;
        if !res.status().is_success() {
            let text = res.text()?;
            // Print better debug when things do fail
            if let Ok(errdata) = serde_json::from_str::<ApiError>(&text) {
                warn!("Unsuccessful: {:?}", errdata);
            } else {
                // In case some parts of ApiError for some reason don't exist..
                warn!("Unsuccessful data: {}", text);
            }
            // Propagate errors properly via reqwest
            let e = res.error_for_status().unwrap_err();
            Err(e.into())
        } else {
            // Should be able to coerce result into Vec<T> at this point
            let mut xs : Vec<T> = vec![];
            let text = res.text()?;
            for l in text.lines() {
                let r = serde_json::from_str(&l).map_err(|e| {
                    warn!("{}", l);
                    Error::from(e)
                })?;
                xs.push(r);
            }
            Ok(xs)

        }
    }
}
