#![allow(non_snake_case)]

use std::fmt::Debug;
use serde::{Deserialize};

use crate::{Result, Error};
use std::collections::BTreeMap;

/// Simplified resource representation
#[derive(Clone, Debug)]
pub struct ApiResource {
    /// API Resource name
    pub resource: String,
    /// API Group
    pub group: String,
    /// Namespace the resources reside
    pub namespace: String,
}

/// Create a list request for a Resource
///
/// Useful to fully re-fetch the state.
pub fn list_all_crd_entries(r: &ApiResource) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = r.group, resource = r.resource, ns = r.namespace);
    let urlstr = url::form_urlencoded::Serializer::new(urlstr).finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}


/// Create watch request for a ApiResource at a given resourceVer
///
/// Should be used continuously
pub fn watch_crd_entries_after(r: &ApiResource, ver: &str) -> Result<http::Request<Vec<u8>>> {
    let urlstr = format!("/apis/{group}/v1/namespaces/{ns}/{resource}?",
        group = r.group, resource = r.resource, ns = r.namespace);
    let mut qp = url::form_urlencoded::Serializer::new(urlstr);

    qp.append_pair("timeoutSeconds", "10");
    qp.append_pair("watch", "true");
    qp.append_pair("resourceVersion", ver);

    let urlstr = qp.finish();
    let mut req = http::Request::get(urlstr);
    req.body(vec![]).map_err(Error::from)
}



// -------------------------------------------------------
// structs + trait relevant to reflector

/// ApiError for when things fail
///
/// This can be parsed into as a fallback in various places
/// `WatchEvents` has a particularly egregious use of it.
#[derive(Deserialize, Debug)]
pub struct ApiError {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    code: u16,
}

/// Events from a watch query
///
/// Should expect a one of these per line from `watch_crd_entries_after`
#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<T> where
  T: Debug + Clone
{
    Added(T),
    Modified(T),
    Deleted(T),
    Error(ApiError),
}


/// Basic resource result wrapper struct
///
/// Expected to be used by `ResourceList` and `WatchEvent`
/// Because it's experimental, it's not exposed outside the crate.
#[derive(Deserialize, Debug, Clone)]
pub struct Resource<T> where
  T: Debug + Clone
{
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    pub spec: T,
}


/// Basic Metadata struct
///
/// Only parses a few fields relevant to a reflector.
/// Because it's experimental, it's not exposed outside the crate.
#[derive(Deserialize, Clone, Debug, Default)]
pub struct Metadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub annotations: BTreeMap<String, String>,
    // TODO: generation?
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resourceVersion: String,
}

/// Basic Resource List
///
/// Expected to be returned by a query from `list_all_crd_entries`
/// Because it's experimental, it's not exposed outside the crate.
#[derive(Deserialize)]
pub struct ResourceList<T> where
  T: Debug + Clone
{
    pub apiVersion: String,
    pub kind: String,
    pub metadata: Metadata,
    #[serde(bound(deserialize = "Vec<T>: Deserialize<'de>"))]
    pub items: Vec<T>,
}
