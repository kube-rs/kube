///! A port of *Optionals from apimachinery/types.go
use crate::{Error, Result};
use serde::Serialize;

/// Common query parameters used in watch/list/delete calls on collections
#[derive(Default, Clone)]
#[allow(missing_docs)]
pub struct ListParams {
    /// A selector to restrict the list of returned objects by their labels.
    ///
    /// Defaults to everything if `None`.
    pub label_selector: Option<String>,

    /// A selector to restrict the list of returned objects by their fields.
    ///
    /// Defaults to everything if `None`.
    pub field_selector: Option<String>,

    /// Timeout for the list/watch call.
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// If unset for a watch call, we will use 290s.
    /// We limit this to 295s due to [inherent watch limitations](https://github.com/kubernetes/kubernetes/issues/6513).
    pub timeout: Option<u32>,

    /// Enables watch events with type "BOOKMARK".
    ///
    /// Servers that do not implement bookmarks ignore this flag and
    /// bookmarks are sent at the server's discretion. Clients should not
    /// assume bookmarks are returned at any specific interval, nor may they
    /// assume the server will send any BOOKMARK event during a session.
    /// If this is not a watch, this field is ignored.
    /// If the feature gate WatchBookmarks is not enabled in apiserver,
    /// this field is ignored.
    pub allow_bookmarks: bool,

    /// Limit the number of results.
    ///
    /// If there are more results, the server will respond with a continue token which can be used to fetch another page
    /// of results. See the [Kubernetes API docs](https://kubernetes.io/docs/reference/using-api/api-concepts/#retrieving-large-results-sets-in-chunks)
    /// for pagination details.
    pub limit: Option<u32>,

    /// Fetch a second page of results.
    ///
    /// After listing results with a limit, a continue token can be used to fetch another page of results.
    pub continue_token: Option<String>,
}

impl ListParams {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(to) = &self.timeout {
            // https://github.com/kubernetes/kubernetes/issues/6513
            if *to >= 295 {
                return Err(Error::RequestValidation(
                    "ListParams::timeout must be < 295s".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Builder interface to ListParams
///
/// Usage:
/// ```
/// use kube::api::ListParams;
/// let lp = ListParams::default()
///     .timeout(60)
///     .labels("kubernetes.io/lifecycle=spot");
/// ```
impl ListParams {
    /// Configure the timeout for list/watch calls
    ///
    /// This limits the duration of the call, regardless of any activity or inactivity.
    /// Defaults to 290s
    pub fn timeout(mut self, timeout_secs: u32) -> Self {
        self.timeout = Some(timeout_secs);
        self
    }

    /// Configure the selector to restrict the list of returned objects by their fields.
    ///
    /// Defaults to everything.
    /// Supports `=`, `==`, `!=`, and can be comma separated: `key1=value1,key2=value2`.
    /// The server only supports a limited number of field queries per type.
    pub fn fields(mut self, field_selector: &str) -> Self {
        self.field_selector = Some(field_selector.to_string());
        self
    }

    /// Configure the selector to restrict the list of returned objects by their labels.
    ///
    /// Defaults to everything.
    /// Supports `=`, `==`, `!=`, and can be comma separated: `key1=value1,key2=value2`.
    pub fn labels(mut self, label_selector: &str) -> Self {
        self.label_selector = Some(label_selector.to_string());
        self
    }

    /// Enables watch bookmarks from the api server if supported
    pub fn allow_bookmarks(mut self) -> Self {
        self.allow_bookmarks = true;
        self
    }

    /// Sets a result limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets a continue token.
    pub fn continue_token(mut self, token: &str) -> Self {
        self.continue_token = Some(token.to_string());
        self
    }
}

/// Common query parameters for put/post calls
#[derive(Default, Clone)]
pub struct PostParams {
    /// Whether to run this as a dry run
    pub dry_run: bool,
    /// fieldManager is a name of the actor that is making changes
    pub field_manager: Option<String>,
}

impl PostParams {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(field_manager) = &self.field_manager {
            // Implement the easy part of validation, in future this may be extended to provide validation as in go code
            // For now it's fine, because k8s API server will return an error
            if field_manager.len() > 128 {
                return Err(Error::RequestValidation(
                    "Failed to validate PostParams::field_manager!".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Common query parameters for patch calls
#[derive(Default, Clone)]
pub struct PatchParams {
    /// Whether to run this as a dry run
    pub dry_run: bool,
    /// Strategy which will be used. Defaults to [`PatchStrategy::Strategic`].
    pub patch_strategy: PatchStrategy,
    /// force Apply requests. Applicable only to [`PatchStrategy::Apply`].
    pub force: bool,
    /// fieldManager is a name of the actor that is making changes. Required for [`PatchStrategy::Apply`]
    /// optional for everything else.
    pub field_manager: Option<String>,
}
impl PatchParams {
    pub(crate) fn validate(&self) -> Result<()> {
        if let Some(field_manager) = &self.field_manager {
            // Implement the easy part of validation, in future this may be extended to provide validation as in go code
            // For now it's fine, because k8s API server will return an error
            if field_manager.len() > 128 {
                return Err(Error::RequestValidation(
                    "Failed to validate PatchParams::field_manager!".into(),
                ));
            }
        }

        if self.patch_strategy != PatchStrategy::Apply && self.force {
            // if not force, all other fields are valid for all types of patch requests
            Err(Error::RequestValidation(
                "Force is applicable only for Apply strategy!".into(),
            ))
        } else {
            Ok(())
        }
    }

    pub(crate) fn populate_qp(&self, qp: &mut url::form_urlencoded::Serializer<String>) {
        if self.dry_run {
            qp.append_pair("dryRun", "true");
        }
        if self.force {
            qp.append_pair("force", "true");
        }
        if let Some(ref field_manager) = self.field_manager {
            qp.append_pair("fieldManager", &field_manager);
        }
    }

    /// Construct `PatchParams` for server-side apply
    pub fn apply(manager: &str) -> Self {
        Self {
            patch_strategy: PatchStrategy::Apply,
            field_manager: Some(manager.into()),
            ..Self::default()
        }
    }

    /// Force the result through on conflicts
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Perform a dryRun only
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// Four different patch types are supported.
///
/// Apply strategy is kinda special
#[derive(Clone, PartialEq)]
pub enum PatchStrategy {
    /// [Server side apply](https://kubernetes.io/docs/reference/using-api/api-concepts/#server-side-apply)
    ///
    /// Requires kubernetes >=1.16
    Apply,
    /// A [JSON merge](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-json-merge-patch-to-update-a-deployment)
    JSON,
    /// A regular merge
    Merge,
    /// A [stategic merge](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-strategic-merge-patch-to-update-a-deployment)
    Strategic,
}

impl std::fmt::Display for PatchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content_type = match &self {
            Self::Apply => "application/apply-patch+yaml",
            Self::JSON => "application/json-patch+json",
            Self::Merge => "application/merge-patch+json",
            Self::Strategic => "application/strategic-merge-patch+json",
        };
        f.write_str(content_type)
    }
}

// Kubectl defaults to Strategic strategy, but doing so will break existing consumers
// so, currently we still default to Merge it may change in future versions
// Strategic merge doesn't work with CRD types https://github.com/kubernetes/kubernetes/issues/52772
impl Default for PatchStrategy {
    fn default() -> Self {
        PatchStrategy::Merge
    }
}

/// Common query parameters for delete calls
#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteParams {
    /// When present, indicates that modifications should not be persisted.
    #[serde(
        serialize_with = "dry_run_all_ser",
        skip_serializing_if = "std::ops::Not::not"
    )]
    pub dry_run: bool,

    /// The duration in seconds before the object should be deleted.
    ///
    /// Value must be non-negative integer. The value zero indicates delete immediately.
    /// If this value is `None`, the default grace period for the specified type will be used.
    /// Defaults to a per object value if not specified. Zero means delete immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grace_period_seconds: Option<u32>,

    /// Whether or how garbage collection is performed.
    ///
    /// The default policy is decided by the existing finalizer set in
    /// `metadata.finalizers`, and the resource-specific default policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagation_policy: Option<PropagationPolicy>,

    /// Condtions that must be fulfilled before a deletion is carried out
    ///
    /// If not possible, a `409 Conflict` status will be returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<Preconditions>,
}

// dryRun serialization differ when used as body parameters and query strings:
// query strings are either true/false
// body params allow only: missing field, or ["All"]
// The latter is a very awkward API causing users to do to
// dp.dry_run = vec!["All".into()];
// just to turn on dry_run..
// so we hide this detail for now.
fn dry_run_all_ser<S>(t: &bool, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    use serde::ser::SerializeTuple;
    match t {
        true => {
            let mut map = s.serialize_tuple(1)?;
            map.serialize_element("All")?;
            map.end()
        }
        false => s.serialize_none(),
    }
}
#[cfg(test)]
mod test {
    use super::DeleteParams;
    #[test]
    fn delete_param_serialize() {
        let mut dp = DeleteParams::default();
        let emptyser = serde_json::to_string(&dp).unwrap();
        println!("emptyser is: {}", emptyser);
        assert_eq!(emptyser, "{}");

        dp.dry_run = true;
        let ser = serde_json::to_string(&dp).unwrap();
        println!("ser is: {}", ser);
        assert_eq!(ser, "{\"dryRun\":[\"All\"]}");
    }
}

/// Preconditions must be fulfilled before an operation (update, delete, etc.) is carried out.
#[derive(Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preconditions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uid: Option<String>,
}

/// Propagation policy when deleting single objects
#[derive(Clone, Debug, Serialize)]
pub enum PropagationPolicy {
    /// Orphan dependents
    Orphan,
    /// Allow the garbage collector to delete the dependents in the background
    Background,
    /// A cascading policy that deletes all dependents in the foreground
    Foreground,
}
