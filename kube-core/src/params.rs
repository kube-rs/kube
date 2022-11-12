//! A port of request parameter *Optionals from apimachinery/types.go
use crate::request::Error;
use serde::Serialize;

/// Common query parameters used in watch/list/delete calls on collections
#[derive(Clone, Debug)]
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
    pub bookmarks: bool,

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

impl Default for ListParams {
    /// Default `ListParams` without any constricting selectors
    fn default() -> Self {
        Self {
            // bookmarks stable since 1.17, and backwards compatible
            bookmarks: true,

            label_selector: None,
            field_selector: None,
            timeout: None,
            limit: None,
            continue_token: None,
        }
    }
}

impl ListParams {
    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Some(to) = &self.timeout {
            // https://github.com/kubernetes/kubernetes/issues/6513
            if *to >= 295 {
                return Err(Error::Validation("ListParams::timeout must be < 295s".into()));
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
    #[must_use]
    pub fn timeout(mut self, timeout_secs: u32) -> Self {
        self.timeout = Some(timeout_secs);
        self
    }

    /// Configure the selector to restrict the list of returned objects by their fields.
    ///
    /// Defaults to everything.
    /// Supports `=`, `==`, `!=`, and can be comma separated: `key1=value1,key2=value2`.
    /// The server only supports a limited number of field queries per type.
    #[must_use]
    pub fn fields(mut self, field_selector: &str) -> Self {
        self.field_selector = Some(field_selector.to_string());
        self
    }

    /// Configure the selector to restrict the list of returned objects by their labels.
    ///
    /// Defaults to everything.
    /// Supports `=`, `==`, `!=`, and can be comma separated: `key1=value1,key2=value2`.
    #[must_use]
    pub fn labels(mut self, label_selector: &str) -> Self {
        self.label_selector = Some(label_selector.to_string());
        self
    }

    /// Disables watch bookmarks to simplify watch handling
    ///
    /// This is not recommended to use with production watchers as it can cause desyncs.
    /// See [#219](https://github.com/kube-rs/kube/issues/219) for details.
    #[must_use]
    pub fn disable_bookmarks(mut self) -> Self {
        self.bookmarks = false;
        self
    }

    /// Sets a result limit.
    #[must_use]
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Sets a continue token.
    #[must_use]
    pub fn continue_token(mut self, token: &str) -> Self {
        self.continue_token = Some(token.to_string());
        self
    }
}

/// The validation directive to use for `fieldValidation` when using server-side apply.
#[derive(Clone, Debug)]
pub enum ValidationDirective {
    /// Strict mode will fail any invalid manifests.
    ///
    /// This will fail the request with a BadRequest error if any unknown fields would be dropped from the
    /// object, or if any duplicate fields are present. The error returned from the server will contain
    /// all unknown and duplicate fields encountered.
    Strict,
    /// Warn mode will return a warning for invalid manifests.
    ///
    /// This will send a warning via the standard warning response header for each unknown field that
    /// is dropped from the object, and for each duplicate field that is encountered. The request will
    /// still succeed if there are no other errors, and will only persist the last of any duplicate fields.
    Warn,
    /// Ignore mode will silently ignore any problems.
    ///
    /// This will ignore any unknown fields that are silently dropped from the object, and will ignore
    /// all but the last duplicate field that the decoder encounters.
    Ignore,
}

impl ValidationDirective {
    /// Returns the string format of the directive
    pub fn as_str(&self) -> &str {
        match self {
            Self::Strict => "Strict",
            Self::Warn => "Warn",
            Self::Ignore => "Ignore",
        }
    }
}

/// Common query parameters for put/post calls
#[derive(Default, Clone, Debug)]
pub struct PostParams {
    /// Whether to run this as a dry run
    pub dry_run: bool,
    /// fieldManager is a name of the actor that is making changes
    pub field_manager: Option<String>,
}

impl PostParams {
    pub(crate) fn populate_qp(&self, qp: &mut form_urlencoded::Serializer<String>) {
        if self.dry_run {
            qp.append_pair("dryRun", "All");
        }
        if let Some(ref fm) = self.field_manager {
            qp.append_pair("fieldManager", fm);
        }
    }

    pub(crate) fn validate(&self) -> Result<(), Error> {
        if let Some(field_manager) = &self.field_manager {
            // Implement the easy part of validation, in future this may be extended to provide validation as in go code
            // For now it's fine, because k8s API server will return an error
            if field_manager.len() > 128 {
                return Err(Error::Validation(
                    "Failed to validate PostParams::field_manager!".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Describes changes that should be applied to a resource
///
/// Takes arbitrary serializable data for all strategies except `Json`.
///
/// We recommend using ([server-side](https://kubernetes.io/blog/2020/04/01/kubernetes-1.18-feature-server-side-apply-beta-2)) `Apply` patches on new kubernetes releases.
///
/// See [kubernetes patch docs](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-json-merge-patch-to-update-a-deployment) for the older patch types.
///
/// Note that patches have different effects on different fields depending on their merge strategies.
/// These strategies are configurable when deriving your [`CustomResource`](https://docs.rs/kube-derive/*/kube_derive/derive.CustomResource.html#customizing-schemas).
///
/// # Creating a patch via serde_json
/// ```
/// use kube::api::Patch;
/// let patch = serde_json::json!({
///     "apiVersion": "v1",
///     "kind": "Pod",
///     "metadata": {
///         "name": "blog"
///     },
///     "spec": {
///         "activeDeadlineSeconds": 5
///     }
/// });
/// let patch = Patch::Apply(&patch);
/// ```
/// # Creating a patch from a type
/// ```
/// use kube::api::Patch;
/// use k8s_openapi::api::rbac::v1::Role;
/// use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
/// let r = Role {
///     metadata: ObjectMeta { name: Some("user".into()), ..ObjectMeta::default() },
///     rules: Some(vec![])
/// };
/// let patch = Patch::Apply(&r);
/// ```
#[non_exhaustive]
#[derive(Debug, PartialEq, Clone)]
pub enum Patch<T: Serialize> {
    /// [Server side apply](https://kubernetes.io/docs/reference/using-api/api-concepts/#server-side-apply)
    ///
    /// Requires kubernetes >= 1.16
    Apply(T),

    /// [JSON patch](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-json-merge-patch-to-update-a-deployment)
    ///
    /// Using this variant will require you to explicitly provide a type for `T` at the moment.
    ///
    /// # Example
    ///
    /// ```
    /// use kube::api::Patch;
    /// let json_patch = json_patch::Patch(vec![]);
    /// let patch = Patch::Json::<()>(json_patch);
    /// ```
    #[cfg(feature = "jsonpatch")]
    #[cfg_attr(docsrs, doc(cfg(feature = "jsonpatch")))]
    Json(json_patch::Patch),

    /// [JSON Merge patch](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-json-merge-patch-to-update-a-deployment)
    Merge(T),
    /// [Strategic JSON Merge patch](https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-strategic-merge-patch-to-update-a-deployment)
    Strategic(T),
}

impl<T: Serialize> Patch<T> {
    pub(crate) fn is_apply(&self) -> bool {
        matches!(self, Patch::Apply(_))
    }

    pub(crate) fn content_type(&self) -> &'static str {
        match &self {
            Self::Apply(_) => "application/apply-patch+yaml",
            #[cfg(feature = "jsonpatch")]
            #[cfg_attr(docsrs, doc(cfg(feature = "jsonpatch")))]
            Self::Json(_) => "application/json-patch+json",
            Self::Merge(_) => "application/merge-patch+json",
            Self::Strategic(_) => "application/strategic-merge-patch+json",
        }
    }
}

impl<T: Serialize> Patch<T> {
    pub(crate) fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        match self {
            Self::Apply(p) => serde_json::to_vec(p),
            #[cfg(feature = "jsonpatch")]
            #[cfg_attr(docsrs, doc(cfg(feature = "jsonpatch")))]
            Self::Json(p) => serde_json::to_vec(p),
            Self::Strategic(p) => serde_json::to_vec(p),
            Self::Merge(p) => serde_json::to_vec(p),
        }
    }
}

/// Common query parameters for patch calls
#[derive(Default, Clone, Debug)]
pub struct PatchParams {
    /// Whether to run this as a dry run
    pub dry_run: bool,
    /// force Apply requests. Applicable only to [`Patch::Apply`].
    pub force: bool,
    /// fieldManager is a name of the actor that is making changes. Required for [`Patch::Apply`]
    /// optional for everything else.
    pub field_manager: Option<String>,
    /// The server-side validation directive to use. Applicable only to [`Patch::Apply`].
    pub field_validation: Option<ValidationDirective>,
}

impl PatchParams {
    pub(crate) fn validate<P: Serialize>(&self, patch: &Patch<P>) -> Result<(), Error> {
        if let Some(field_manager) = &self.field_manager {
            // Implement the easy part of validation, in future this may be extended to provide validation as in go code
            // For now it's fine, because k8s API server will return an error
            if field_manager.len() > 128 {
                return Err(Error::Validation(
                    "Failed to validate PatchParams::field_manager!".into(),
                ));
            }
        }
        if self.force && !patch.is_apply() {
            return Err(Error::Validation(
                "PatchParams::force only works with Patch::Apply".into(),
            ));
        }
        Ok(())
    }

    pub(crate) fn populate_qp(&self, qp: &mut form_urlencoded::Serializer<String>) {
        if self.dry_run {
            qp.append_pair("dryRun", "All");
        }
        if self.force {
            qp.append_pair("force", "true");
        }
        if let Some(ref fm) = self.field_manager {
            qp.append_pair("fieldManager", fm);
        }
        if let Some(sv) = &self.field_validation {
            qp.append_pair("fieldValidation", sv.as_str());
        }
    }

    /// Construct `PatchParams` for server-side apply
    #[must_use]
    pub fn apply(manager: &str) -> Self {
        Self {
            field_manager: Some(manager.into()),
            ..Self::default()
        }
    }

    /// Force the result through on conflicts
    ///
    /// NB: Force is a concept restricted to the server-side [`Patch::Apply`].
    #[must_use]
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Perform a dryRun only
    #[must_use]
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Set the validation directive for `fieldValidation` during server-side apply.
    pub fn validation(mut self, vd: ValidationDirective) -> Self {
        self.field_validation = Some(vd);
        self
    }

    /// Set the validation directive to `Ignore`
    #[must_use]
    pub fn validation_ignore(self) -> Self {
        self.validation(ValidationDirective::Ignore)
    }

    /// Set the validation directive to `Warn`
    #[must_use]
    pub fn validation_warn(self) -> Self {
        self.validation(ValidationDirective::Warn)
    }

    /// Set the validation directive to `Strict`
    #[must_use]
    pub fn validation_strict(self) -> Self {
        self.validation(ValidationDirective::Strict)
    }
}

/// Common query parameters for delete calls
#[derive(Default, Clone, Serialize, Debug)]
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

impl DeleteParams {
    /// Construct `DeleteParams` with `PropagationPolicy::Background`.
    ///
    /// This allows the garbage collector to delete the dependents in the background.
    pub fn background() -> Self {
        Self {
            propagation_policy: Some(PropagationPolicy::Background),
            ..Self::default()
        }
    }

    /// Construct `DeleteParams` with `PropagationPolicy::Foreground`.
    ///
    /// This is a cascading policy that deletes all dependents in the foreground.
    pub fn foreground() -> Self {
        Self {
            propagation_policy: Some(PropagationPolicy::Foreground),
            ..Self::default()
        }
    }

    /// Construct `DeleteParams` with `PropagationPolicy::Orphan`.
    ///
    ///
    /// This orpans the dependents.
    pub fn orphan() -> Self {
        Self {
            propagation_policy: Some(PropagationPolicy::Orphan),
            ..Self::default()
        }
    }

    /// Perform a dryRun only
    #[must_use]
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Set the duration in seconds before the object should be deleted.
    #[must_use]
    pub fn grace_period(mut self, secs: u32) -> Self {
        self.grace_period_seconds = Some(secs);
        self
    }

    /// Set the condtions that must be fulfilled before a deletion is carried out.
    #[must_use]
    pub fn preconditions(mut self, preconditions: Preconditions) -> Self {
        self.preconditions = Some(preconditions);
        self
    }

    pub(crate) fn is_default(&self) -> bool {
        !self.dry_run
            && self.grace_period_seconds.is_none()
            && self.propagation_policy.is_none()
            && self.preconditions.is_none()
    }
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
    use super::{DeleteParams, PatchParams};
    #[test]
    fn delete_param_serialize() {
        let mut dp = DeleteParams::default();
        let emptyser = serde_json::to_string(&dp).unwrap();
        //println!("emptyser is: {}", emptyser);
        assert_eq!(emptyser, "{}");

        dp.dry_run = true;
        let ser = serde_json::to_string(&dp).unwrap();
        //println!("ser is: {}", ser);
        assert_eq!(ser, "{\"dryRun\":[\"All\"]}");
    }

    #[test]
    fn delete_param_constructors() {
        let dp_background = DeleteParams::background();
        let ser = serde_json::to_value(&dp_background).unwrap();
        assert_eq!(ser, serde_json::json!({"propagationPolicy": "Background"}));

        let dp_foreground = DeleteParams::foreground();
        let ser = serde_json::to_value(&dp_foreground).unwrap();
        assert_eq!(ser, serde_json::json!({"propagationPolicy": "Foreground"}));

        let dp_orphan = DeleteParams::orphan();
        let ser = serde_json::to_value(&dp_orphan).unwrap();
        assert_eq!(ser, serde_json::json!({"propagationPolicy": "Orphan"}));
    }

    #[test]
    fn patch_param_serializes_field_validation() {
        let pp = PatchParams::default().validation_ignore();
        let mut qp = form_urlencoded::Serializer::new(String::from("some/resource?"));
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        assert_eq!(String::from("some/resource?&fieldValidation=Ignore"), urlstr);

        let pp = PatchParams::default().validation_warn();
        let mut qp = form_urlencoded::Serializer::new(String::from("some/resource?"));
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        assert_eq!(String::from("some/resource?&fieldValidation=Warn"), urlstr);

        let pp = PatchParams::default().validation_strict();
        let mut qp = form_urlencoded::Serializer::new(String::from("some/resource?"));
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        assert_eq!(String::from("some/resource?&fieldValidation=Strict"), urlstr);
    }
}

/// Preconditions must be fulfilled before an operation (update, delete, etc.) is carried out.
#[derive(Default, Clone, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Preconditions {
    /// Specifies the target ResourceVersion
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_version: Option<String>,
    /// Specifies the target UID
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
