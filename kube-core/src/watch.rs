//! Types for the watch api
//!
//! See <https://kubernetes.io/docs/reference/using-api/api-concepts/#efficient-detection-of-changes>

use crate::{error::ErrorResponse, metadata::TypeMeta};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
/// A raw event returned from a watch query
///
/// Note that a watch query returns many of these as newline separated JSON.
#[derive(Deserialize, Serialize, Clone)]
#[serde(tag = "type", content = "object", rename_all = "UPPERCASE")]
pub enum WatchEvent<K> {
    /// Resource was added
    Added(K),
    /// Resource was modified
    Modified(K),
    /// Resource was deleted
    Deleted(K),
    /// Resource bookmark. `Bookmark` is a slimmed down `K` due to [#285](https://github.com/kube-rs/kube/issues/285).
    ///
    /// From [Watch bookmarks](https://kubernetes.io/docs/reference/using-api/api-concepts/#watch-bookmarks).
    ///
    /// NB: This became Beta first in Kubernetes 1.16.
    Bookmark(Bookmark),
    /// There was some kind of error
    Error(ErrorResponse),
}

impl<K> Debug for WatchEvent<K> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            WatchEvent::Added(_) => write!(f, "Added event"),
            WatchEvent::Modified(_) => write!(f, "Modified event"),
            WatchEvent::Deleted(_) => write!(f, "Deleted event"),
            WatchEvent::Bookmark(_) => write!(f, "Bookmark event"),
            WatchEvent::Error(e) => write!(f, "Error event: {:?}", e),
        }
    }
}

/// Slimed down K for [`WatchEvent::Bookmark`] due to [#285](https://github.com/kube-rs/kube/issues/285).
///
/// Can only be relied upon to have metadata with resource version.
/// Bookmarks contain apiVersion + kind + basically empty metadata.
#[derive(Serialize, Deserialize, Clone)]
pub struct Bookmark {
    /// apiVersion + kind
    #[serde(flatten)]
    pub types: TypeMeta,

    /// Basically empty metadata
    pub metadata: BookmarkMeta,
}

/// Slimed down Metadata for WatchEvent::Bookmark
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkMeta {
    /// The only field we need from a Bookmark event.
    pub resource_version: String,
}
