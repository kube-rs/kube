//! High-level utilities for runtime API discovery.

use crate::{Client, Result};
pub use kube_core::discovery::{verbs, ApiCapabilities, ApiResource, Scope};
use kube_core::gvk::GroupVersionKind;
use std::collections::HashMap;
mod apigroup;
pub mod oneshot;
pub use apigroup::ApiGroup;
mod parse;

// re-export one-shots
pub use oneshot::{group, pinned_group, pinned_kind};

/// How the Discovery client decides what api groups to scan
enum DiscoveryMode {
    /// Only allow explicitly listed apigroups
    Allow(Vec<String>),
    /// Allow all apigroups except the ones listed
    Block(Vec<String>),
}

impl DiscoveryMode {
    fn is_queryable(&self, group: &String) -> bool {
        match &self {
            Self::Allow(allowed) => allowed.contains(group),
            Self::Block(blocked) => !blocked.contains(group),
        }
    }
}

/// A caching client for running API discovery against the Kubernetes API.
///
/// This simplifies the required querying and type matching, and stores the responses
/// for each discovered api group and exposes helpers to access them.
///
/// The discovery process varies in complexity depending on:
/// - how much you know about the kind(s) and group(s) you are interested in
/// - how many groups you are interested in
///
/// Discovery can be performed on:
/// - all api groups (default)
/// - a subset of api groups (by setting Discovery::filter)
///
/// To make use of discovered apis, extract one or more [`ApiGroup`]s from it,
/// or resolve a precise one using [`Discovery::resolve_gvk`](crate::discovery::Discovery::resolve_gvk).
///
/// If caching of results is __not required__, then a simpler [`oneshot`](crate::discovery::oneshot) discovery system can be used.
///
/// [`ApiGroup`]: crate::discovery::ApiGroup
#[cfg_attr(docsrs, doc(cfg(feature = "client")))]
pub struct Discovery {
    client: Client,
    groups: HashMap<String, ApiGroup>,
    mode: DiscoveryMode,
}

/// Caching discovery interface
///
/// Builds an internal map of its cache
impl Discovery {
    /// Construct a caching api discovery client
    #[must_use]
    pub fn new(client: Client) -> Self {
        let groups = HashMap::new();
        let mode = DiscoveryMode::Block(vec![]);
        Self { client, groups, mode }
    }

    /// Configure the discovery client to only look for the listed apigroups
    #[must_use]
    pub fn filter(mut self, allow: &[&str]) -> Self {
        self.mode = DiscoveryMode::Allow(allow.iter().map(ToString::to_string).collect());
        self
    }

    /// Configure the discovery client to look for all apigroups except the listed ones
    #[must_use]
    pub fn exclude(mut self, deny: &[&str]) -> Self {
        self.mode = DiscoveryMode::Block(deny.iter().map(ToString::to_string).collect());
        self
    }

    /// Runs or re-runs the configured discovery algorithm and updates/populates the cache
    ///
    /// The cache is empty cleared when this is started. By default, every api group found is checked,
    /// causing `N+2` queries to the api server (where `N` is number of api groups).
    ///
    /// ```no_run
    /// use kube::{Client, api::{Api, DynamicObject}, discovery::{Discovery, verbs, Scope}, ResourceExt};
    /// #[tokio::main]
    /// async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let client = Client::try_default().await?;
    ///     let discovery = Discovery::new(client.clone()).run().await?;
    ///     for group in discovery.groups() {
    ///         for (ar, caps) in group.recommended_resources() {
    ///             if !caps.supports_operation(verbs::LIST) {
    ///                 continue;
    ///             }
    ///             let api: Api<DynamicObject> = Api::all_with(client.clone(), &ar);
    ///             // can now api.list() to emulate kubectl get all --all
    ///             for obj in api.list(&Default::default()).await? {
    ///                 println!("{} {}: {}", ar.api_version, ar.kind, obj.name());
    ///             }
    ///         }
    ///     }
    ///     Ok(())
    /// }
    /// ```
    /// See a bigger example in [examples/dynamic.api](https://github.com/kube-rs/kube/blob/main/examples/dynamic_api.rs)
    pub async fn run(mut self) -> Result<Self> {
        self.groups.clear();
        let api_groups = self.client.list_api_groups().await?;
        // query regular groups + crds under /apis
        for g in api_groups.groups {
            let key = g.name.clone();
            if self.mode.is_queryable(&key) {
                let apigroup = ApiGroup::query_apis(&self.client, g).await?;
                self.groups.insert(key, apigroup);
            }
        }
        // query core versions under /api
        let corekey = ApiGroup::CORE_GROUP.to_string();
        if self.mode.is_queryable(&corekey) {
            let coreapis = self.client.list_core_api_versions().await?;
            let apigroup = ApiGroup::query_core(&self.client, coreapis).await?;
            self.groups.insert(corekey, apigroup);
        }
        Ok(self)
    }
}

/// Interface to the Discovery cache
impl Discovery {
    /// Returns iterator over all served groups
    pub fn groups(&self) -> impl Iterator<Item = &ApiGroup> {
        self.groups.values()
    }

    /// Returns a sorted vector of all served groups
    ///
    /// This vector is in kubectl's normal alphabetical group order
    pub fn groups_alphabetical(&self) -> Vec<&ApiGroup> {
        let mut values: Vec<_> = self.groups().collect();
        // collect to maintain kubectl order of groups
        values.sort_by_key(|g| g.name());
        values
    }

    /// Returns the [`ApiGroup`] for a given group if served
    pub fn get(&self, group: &str) -> Option<&ApiGroup> {
        self.groups.get(group)
    }

    /// Check if a group is served by the apiserver
    pub fn has_group(&self, group: &str) -> bool {
        self.groups.contains_key(group)
    }

    /// Finds an [`ApiResource`] and its [`ApiCapabilities`] after discovery by matching a GVK
    ///
    /// This is for quick extraction after having done a complete discovery.
    /// If you are only interested in a single kind, consider [`oneshot::pinned_kind`](crate::discovery::pinned_kind).
    pub fn resolve_gvk(&self, gvk: &GroupVersionKind) -> Option<(ApiResource, ApiCapabilities)> {
        self.get(&gvk.group)?
            .versioned_resources(&gvk.version)
            .into_iter()
            .find(|res| res.0.kind == gvk.kind)
    }
}
