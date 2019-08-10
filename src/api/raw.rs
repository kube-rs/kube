use crate::{Result, ErrorKind};
use failure::ResultExt;

/// RawApi generation data
///
/// This data defines the urls used by kubernetes' APIs.
/// This struct is client agnostic, and can be passed to an Informer or a Reflector.
///
/// Can be used directly with a client.
/// When data is PUT/POST/PATCH'd this struct requires serialized raw bytes.
#[derive(Clone, Debug)]
pub struct RawApi {
    /// API Resource name
    pub resource: String,
    /// API Group
    pub group: String,
    /// Namespace the resources reside
    pub namespace: Option<String>,
    /// API version of the resource
    pub version: String,
    /// Name of the api prefix (api or apis typically)
    pub prefix: String,
    // extra properties for sub resources
}

impl Default for RawApi {
    fn default() -> Self {
        Self {
            resource: "pods".into(), // had to pick something here
            namespace: None,
            group: "".into(),
            version: "v1".into(),
            prefix: "apis".into(), // seems most common
        }
    }
}

/// Constructors for most kubernetes objects
///
/// Don't see all objects in here? Please submit a PR.
/// You can extract the data needed from the [openapi spec](https://docs.rs/k8s-openapi/0.4.0/k8s_openapi/api/).
#[allow(non_snake_case)]
impl RawApi {
    /// Set as namespaced resource within a specified namespace
    pub fn within(mut self, ns: &str) -> Self {
        match self.resource.as_ref() {
            "nodes" | "namespaces" | "customresourcedefinitions" =>
                panic!("{} is not a namespace scoped resource", self.resource),
            _ => {},
        }
        self.namespace = Some(ns.to_string());
        self
    }
    /// Set the api group of a resource manually
    ///
    /// Can be used to set legacy versions like "extensions" for old Deployments
    pub fn group(mut self, group: &str) -> Self {
        self.group = group.to_string();
        self
    }
    /// Set the version of an api group manually
    ///
    /// Can be used to set legacy versions like "v1beta1" for old Deployments
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }
    /// Stable namespace resource constructor
    pub fn v1Namespace() -> Self {
        Self {
            group: "".into(),
            resource: "namespaces".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }
    /// Stable deployment resource constructor
    pub fn v1Deployment() -> Self {
        Self {
            group: "apps".into(),
            resource: "deployments".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    /// Stable pod resource constructor
    pub fn v1Pod() -> Self {
        Self {
            group: "".into(),
            resource: "pods".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }
    /// Stable daemonset resource constructor
    pub fn v1DaemonSet() -> Self {
        Self {
            group: "apps".into(),
            resource: "daemonsets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    /// Stable replicaset resource constructor
    pub fn v1ReplicaSet() -> Self {
        Self {
            group: "apps".into(),
            resource: "replicasets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    /// Stable node resource constructor
    pub fn v1Node() -> Self {
        Self {
            group: "".into(),
            resource: "nodes".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }
    /// Stable statefulset resource constructor
    pub fn v1Statefulset() -> Self {
        Self {
            group: "apps".into(),
            resource: "statefulsets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    // Stable event resource constructor
    pub fn v1Event() -> Self {
        Self {
            group: "".into(),
            resource: "events".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }

    // Stable Service resource constructor
    pub fn v1Service() -> Self {
        Self {
            group: "".into(),
            resource: "services".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }

    // Stable Secret resource constructor
    pub fn v1Secret() -> Self {
        Self {
            group: "".into(),
            resource: "secrets".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }

    // Stable ConfigMap resource constructor
    pub fn v1ConfigMap() -> Self {
        Self {
            group: "".into(),
            resource: "configmaps".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }

    pub fn v1Job() -> Self {
        Self {
            group: "batch".into(),
            resource: "jobs".into(),
            prefix: "apis".into(),
            version: "v1".into(),
            ..Default::default()
        }
    }

    // Stable PersistentVolumeClaim resource constructor
    pub fn v1PersistentVolumeClaim() -> Self {
        Self {
            group: "".into(),
            resource: "persistentvolumeclaims".into(),
            prefix: "api".into(),
            version: "v1".into(),
            ..Default::default()
        }
    }

    // Stable PersistentVolume resource constructor
    pub fn v1PersistentVolume() -> Self {
        Self {
            group: "".into(),
            resource: "persistentvolumes".into(),
            prefix: "api".into(),
            version: "v1".into(),
            ..Default::default()
        }
    }

    // Stable NetworkPolicy resource constructor
    pub fn v1NetworkPolicy() -> Self {
        Self {
            group: "networking.k8s.io".into(),
            resource: "networkpolicies".into(),
            prefix: "apis".into(),
            version: "v1".into(),
            ..Default::default()
        }
    }

    // Stable HorizontalPodAutoscaler resource constructor
    pub fn v1HorizontalPodAutoscaler() -> Self {
        Self {
            group: "autoscaling".into(),
            resource: "horizontalpodautoscalers".into(),
            prefix: "apis".into(),
            version: "v1".into(),
            ..Default::default()
        }
    }

    /// Custom resource definition constructor
    pub fn v1beta1CustomResourceDefinition() -> Self {
        Self {
            group: "apiextensions.k8s.io".into(),
            resource: "customresourcedefinitions".into(),
            prefix: "apis".into(),
            version: "v1beta1".into(), // latest available in 1.14.0
            ..Default::default()
        }
    }
    /// Instance of a CRD
    ///
    /// The version, and group must be set by the user:
    ///
    /// ```
    /// use kube::api::RawApi;
    /// let foos = RawApi::customResource("foos") // <.spec.name>
    ///    .group("clux.dev") // <.spec.group>
    ///    .version("v1");
    /// ```
    pub fn customResource(name: &str) -> Self {
        Self {
            resource: name.into(),
            ..Default::default()
        }
    }
}

// -------------------------------------------------------

impl RawApi {
    fn make_url(&self) -> String {
        let pref = if self.prefix == "" { "".into() } else { format!("{}/", self.prefix) };
        let g = if self.group == "" { "".into() } else { format!("{}/", self.group) };
        let n = if let Some(ns) = &self.namespace { format!("namespaces/{}/", ns) } else { "".into() };

        format!("/{prefix}{group}{version}/{namespaces}{resource}",
            prefix = pref,
            group = g,
            version = self.version,
            namespaces = n,
            resource = self.resource,
        )
    }
}

/// Common query parameters used in watch/list/delete calls on collections
///
/// Constructed internally with a builder on Informer and Reflector,
/// but can be passed to the helper function of RawRawApi.
#[derive(Default, Clone)]
pub struct ListParams {
    pub field_selector: Option<String>,
    pub include_uninitialized: bool,
    pub label_selector: Option<String>,
    pub timeout: Option<u32>
}

/// Common query parameters for put/post calls
#[derive(Default, Clone)]
pub struct PostParams {
    pub dry_run: bool,
}

/// Common query parameters for patch calls
#[derive(Default, Clone)]
pub struct PatchParams {
    pub dry_run: bool,
    /// Strategy which will be used. Defaults to `PatchStrategy::Strategic`
    pub patch_strategy: PatchStrategy,
    /// force Apply requests. Applicable only to `PatchStrategy::Apply`
    pub force: bool,
    /// fieldManager is a name of the actor that is making changes. Required for `PatchStrategy::Apply`
    /// optional for everything else
    pub field_manager: Option<String>
}

impl PatchParams {
    fn validate(&self) -> Result<()> {
        if let Some(field_manager) = &self.field_manager {
            // Implement the easy part of validation, in future this may be extended to provide validation as in go code
            // For now it's fine, because k8s API server will return an error
            if field_manager.len() > 128 {
            return Err(ErrorKind::RequestValidation("Failed to validate PatchParameters::field_manager!".to_owned()).into())
            }
        }

        if self.patch_strategy != PatchStrategy::Apply && self.force {
             // if not force, all other fields are valid for all types of patch requests
            Err(ErrorKind::RequestValidation("Force is applicable only for Apply strategy!".to_owned()).into())
        } else {
            Ok(())
        }
    }

    fn populate_qp(&self, qp: &mut url::form_urlencoded::Serializer<String>) {
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
}


/// For patch different patch types are supported. See https://kubernetes.io/docs/tasks/run-application/update-api-object-kubectl-patch/#use-a-json-merge-patch-to-update-a-deployment
/// Apply strategy is kinda special
#[derive(Clone, PartialEq)]
pub enum PatchStrategy {
    Apply,
    JSON,
    Merge,
    Strategic
}

impl std::fmt::Display for PatchStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let content_type = match &self {
            PatchStrategy::Apply => "application/apply-patch+yaml",
            PatchStrategy::JSON => "application/json-patch+json",
            PatchStrategy::Merge => "application/merge-patch+json",
            PatchStrategy::Strategic => "application/strategic-merge-patch+json"
        };
        f.write_str(content_type)
    }
}

// Kubectl defaults to Strategic strategy, but doing so will break existing consumers
// so, currently we still default to Merge it may change in future versions
// Strategic merge doesn't work with CRD types https://github.com/kubernetes/kubernetes/issues/52772
impl Default for PatchStrategy {
    fn default() -> Self { PatchStrategy::Merge }
}

/// Common query parameters for delete calls
#[derive(Default, Clone)]
pub struct DeleteParams {
    /// When present, indicates that modifications should not be persisted.
    ///
    /// An invalid or unrecognized dryRun directive will result in an error response
    /// and no further processing of the request.
    pub dry_run: bool,
    /// The duration in seconds before the object should be deleted.
    ///
    /// Value must be non-negative integer. The value zero indicates delete immediately.
    /// If this value is None, the default grace period for the specified type will be used.
    /// Defaults to a per object value if not specified. Zero means delete immediately.
    pub grace_period_seconds: Option<u32>,
    /// Whether or how garbage collection is performed.
    ///
    /// The default policy is decided by the existing finalizer set in
    /// metadata.finalizers, and the resource-specific default policy.
    pub propagation_policy: Option<PropagationPolicy>,
}

/// Propagation policy when deleting single objects
#[derive(Clone, Debug)]
pub enum PropagationPolicy {
    /// Orphan dependents
    Orphan,
    /// Allow the garbage collector to delete the dependents in the background
    Background,
    /// A cascading policy that deletes all dependents in the foreground
    Foreground,
}

#[derive(Default, Clone, Debug)]
pub struct LogParams {
    /// The container for which to stream logs. Defaults to only container if there is one container in the pod.
    pub container: Option<String>,
    /// Follow the log stream of the pod. Defaults to false.
    pub follow: bool,
    /// If set, the number of bytes to read from the server before terminating the log output.
    /// This may not display a complete final line of logging, and may return slightly more or slightly less than the specified limit.
    pub limit_bytes: Option<i64>,
    /// If 'true', then the output is pretty printed.
    pub pretty: bool,
    /// Return previous terminated container logs. Defaults to false.
    pub previous: bool,
    /// A relative time in seconds before the current time from which to show logs.
    /// If this value precedes the time a pod was started, only logs since the pod start will be returned.
    /// If this value is in the future, no logs will be returned. Only one of sinceSeconds or sinceTime may be specified.
    pub since_seconds: Option<i64>,
    /// If set, the number of lines from the end of the logs to show.
    /// If not specified, logs are shown from the creation of the container or sinceSeconds or sinceTime
    pub tail_lines: Option<i64>,
    /// If true, add an RFC3339 or RFC3339Nano timestamp at the beginning of every line of log output. Defaults to false.
    pub timestamps: bool,
}

/// Convenience methods found from API conventions
impl RawApi {
    /// List a collection of a resource
    pub fn list(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if lp.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Create a minimial list request to seed an initial resourceVersion
    pub(crate) fn list_zero_resource_entries(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        qp.append_pair("limit", "1"); // can't have 0..
        if lp.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        // rest of lp doesn't matter here - we just need a resourceVersion
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Watch a resource at a given version
    pub fn watch(&self, lp: &ListParams, ver: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        qp.append_pair("watch", "true");
        qp.append_pair("resourceVersion", ver);

        qp.append_pair("timeoutSeconds", &lp.timeout.unwrap_or(10).to_string());
        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if lp.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Get a single instance
    pub fn get(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name;
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Create an instance of a resource
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::post(urlstr);
        Ok(req.body(data).context(ErrorKind::RequestBuild)?)
    }

    /// Delete an instance of a resource
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/"+ name + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if dp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        if let Some(grace) = dp.grace_period_seconds {
            qp.append_pair("gracePeriodSeconds", &grace.to_string());
        }
        if let Some(ref prop) = dp.propagation_policy {
            qp.append_pair("propagationPolicy", &format!("{:?}", prop));
        }
        let urlstr = qp.finish();
        let mut req = http::Request::delete(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Delete a collection of a resource
    pub fn delete_collection(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if lp.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }
        let urlstr = qp.finish();
        let mut req = http::Request::delete(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Patch an instance of a resource
    ///
    /// Requires a serialized merge-patch+json at the moment.
    pub fn patch(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let base_url = self.make_url() + "/" + name + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        Ok(http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch).context(ErrorKind::RequestBuild)?)
    }

    /// Replace an instance of a resource
    ///
    /// Requires metadata.resourceVersion set in data
    pub fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        Ok(req.body(data).context(ErrorKind::RequestBuild)?)
    }

    /// Get an instance of the scale subresource
    pub fn get_scale(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/scale";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Patch an instance of the scale subresource
    pub fn patch_scale(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let base_url = self.make_url() + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        Ok(http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch).context(ErrorKind::RequestBuild)?)
    }

    /// Replace an instance of the scale subresource
    pub fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        Ok(req.body(data).context(ErrorKind::RequestBuild)?)
    }

    /// Get an instance of the status subresource
    pub fn get_status(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/status";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }

    /// Patch an instance of the status subresource
    pub fn patch_status(&self, name: &str, pp: &PatchParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let base_url = self.make_url() + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        Ok(http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch).context(ErrorKind::RequestBuild)?)
    }

    /// Replace an instance of the status subresource
    pub fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        Ok(req.body(data).context(ErrorKind::RequestBuild)?)
    }
}

impl RawApi {
    /// Get a pod logs
    pub fn log(&self, name: &str, lp: &LogParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/" + "log";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        if let Some(container) = &lp.container {
            qp.append_pair("container", &container);
        }

        if lp.follow {
            qp.append_pair("follow", "true");
        }

        if let Some(limitBytes) = &lp.limit_bytes {
            qp.append_pair("limitBytes", &limitBytes.to_string());
        }

        if lp.pretty {
            qp.append_pair("pretty", "true");
        }

        if lp.previous {
            qp.append_pair("previous", "true");
        }

        if let Some(sinceSeconds) = &lp.since_seconds {
            qp.append_pair("sinceSeconds", &sinceSeconds.to_string());
        }

        if let Some(tailLines) = &lp.tail_lines {
            qp.append_pair("tailLines", &tailLines.to_string());
        }

        if lp.timestamps {
            qp.append_pair("timestamps", "true");
        }

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        Ok(req.body(vec![]).context(ErrorKind::RequestBuild)?)
    }
}

#[test]
fn list_path(){
    let r = RawApi::v1Deployment().within("ns");
    let gp = ListParams::default();
    let req = r.list(&gp).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
}
#[test]
fn watch_path() {
    let r = RawApi::v1Pod().within("ns");
    let gp = ListParams::default();
    let req = r.watch(&gp, "0").unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods?&watch=true&resourceVersion=0&timeoutSeconds=10");
}
#[test]
fn replace_path(){
    let r = RawApi::v1DaemonSet();
    let pp = PostParams { dry_run: true, ..Default::default() };
    let req = r.replace("myds", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/daemonsets/myds?&dryRun=All");
}
#[test]
fn create_path() {
    let r = RawApi::v1ReplicaSet().within("ns");
    let pp = PostParams::default();
    let req = r.create(&pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
}
#[test]
fn delete_path() {
    let r = RawApi::v1ReplicaSet().within("ns");
    let dp = DeleteParams::default();
    let req = r.delete("myrs", &dp).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
    assert_eq!(req.method(), "DELETE")
}

#[test]
fn delete_collection_path() {
    let r = RawApi::v1ReplicaSet().within("ns");
    let lp = ListParams::default();
    let req = r.delete_collection(&lp).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets");
    assert_eq!(req.method(), "DELETE")
}

#[test]
fn namespace_path() { // weird object compared to other v1
    let r = RawApi::v1Namespace();
    let gp = ListParams::default();
    let req = r.list(&gp).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces")
}

#[test]
fn patch_params_validation() {
    let pp = PatchParams::default();
    assert!(pp.validate().is_ok(), "default params should always be valid");

    let patch_strategy_apply_true = PatchParams {
        patch_strategy: PatchStrategy::Merge,
        force: true,
        ..Default::default()
    };
    assert!(patch_strategy_apply_true.validate().is_err(), "Merge strategy shouldn't be valid if `force` set to true");
}

// subresources with weird version accuracy
#[test]
fn patch_status_path(){
    let r = RawApi::v1Node();
    let pp = PatchParams::default();
    let req = r.patch_status("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
    assert_eq!(req.headers().get("Content-Type").unwrap().to_str().unwrap(), format!("{}", PatchStrategy::Merge));
    assert_eq!(req.method(), "PATCH");
}
#[test]
fn replace_status_path(){
    let r = RawApi::v1Node();
    let pp = PostParams::default();
    let req = r.replace_status("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
    assert_eq!(req.method(), "PUT");
}

#[test]
fn create_custom_resource() {
    let r = RawApi::customResource("foos")
        .group("clux.dev").version("v1")
        .within("myns");
    let pp = PostParams::default();
    let req = r.create(&pp, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/clux.dev/v1/namespaces/myns/foos?"
    );
    let patch_params = PatchParams::default();
    let req = r.patch("baz", &patch_params, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/clux.dev/v1/namespaces/myns/foos/baz?"
    );
    assert_eq!(req.method(), "PATCH");
}

#[test]
fn replace_status() {
    let r = RawApi::v1beta1CustomResourceDefinition();
    let pp = PostParams::default();
    let req = r.replace_status("mycrd.domain.io", &pp, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/apiextensions.k8s.io/v1beta1/customresourcedefinitions/mycrd.domain.io/status?"
    );
}
#[test]
fn get_scale_path(){
    let r = RawApi::v1Node();
    let req = r.get_scale("mynode").unwrap();
    assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale");
    assert_eq!(req.method(), "GET");
}
#[test]
fn patch_scale_path(){
    let r = RawApi::v1Node();
    let pp = PatchParams::default();
    let req = r.patch_scale("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
    assert_eq!(req.method(), "PATCH");
}
#[test]
fn replace_scale_path(){
    let r = RawApi::v1Node();
    let pp = PostParams::default();
    let req = r.replace_scale("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
    assert_eq!(req.method(), "PUT");
}

#[test]
#[should_panic]
fn global_resources_not_namespaceable(){
    RawApi::v1Node().within("ns");
}
