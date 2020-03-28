use crate::{Error, Result};
use inflector::string::pluralize::to_plural;

/// The Resource information needed to operate a kubernetes client
#[derive(Clone, Debug)]
pub struct Resource {
    /// The API version of the resource.
    ///
    /// This is a composite of Resource::GROUP and Resource::VERSION
    /// (eg "apiextensions.k8s.io/v1beta1")
    /// or just the version for resources without a group (eg "v1").
    /// This is the string used in the apiVersion field of the resource's serialized form.
    pub api_version: String,

    /// The group of the resource
    ///
    /// or the empty string if the resource doesn't have a group.
    pub group: String,

    /// The kind of the resource.
    ///
    /// This is the string used in the kind field of the resource's serialized form.
    pub kind: String,

    /// The version of the resource.
    pub version: String,

    pub scope: ResourceScope,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResourceScope {
    Cluster,
    Namespace(String),
    All,
}


// Try Arnavion's first suggestion
pub trait ClusterScopedResource: k8s_openapi::Resource { }
pub trait NamespaceScopedResource: k8s_openapi::Resource { }
use k8s::{
    admissionregistration::v1beta1 as adregv1beta1,
    apps::v1 as appsv1,
    authorization::v1 as authv1,
    autoscaling::v1 as autoscalingv1,
    batch::v1beta1 as batchv1beta1,
    core::v1 as corev1,
    extensions::v1beta1 as extsv1beta1,
    networking::{v1 as networkingv1, v1beta1 as networkingv1beta1},
    rbac::v1 as rbacv1,
    storage::v1 as storagev1,
};
use k8s_openapi::api as k8s;
impl NamespaceScopedResource for corev1::Secret {}
impl NamespaceScopedResource for rbacv1::Role {}
impl NamespaceScopedResource for batchv1beta1::CronJob {}
impl NamespaceScopedResource for autoscalingv1::HorizontalPodAutoscaler {}
impl NamespaceScopedResource for networkingv1::NetworkPolicy {}
impl NamespaceScopedResource for extsv1beta1::Ingress {}
impl NamespaceScopedResource for appsv1::Deployment {}
impl NamespaceScopedResource for corev1::Pod {}
impl NamespaceScopedResource for appsv1::ReplicaSet {}
impl NamespaceScopedResource for networkingv1beta1::Ingress {}
impl NamespaceScopedResource for appsv1::DaemonSet {}

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiextsv1;
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiextsv1beta1;

impl ClusterScopedResource for storagev1::VolumeAttachment {}
impl ClusterScopedResource for adregv1beta1::ValidatingWebhookConfiguration {}
impl ClusterScopedResource for authv1::SelfSubjectRulesReview {}
impl ClusterScopedResource for apiextsv1::CustomResourceDefinition {}
impl ClusterScopedResource for corev1::Namespace {}
impl ClusterScopedResource for apiextsv1beta1::CustomResourceDefinition {}
impl ClusterScopedResource for corev1::Node {}



impl Resource {
    /// Cluster level resources,
    pub fn cluster<K: ClusterScopedResource>() -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            scope: ResourceScope::Cluster,
        }
    }

    /// Namespaced resources viewed across all namespaces
    ///
    /// This does not let you read / get individual objects
    /// because the resources still need to know the underlying namespace.
    pub fn all<K: NamespaceScopedResource>() -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            scope: ResourceScope::All
        }
    }

    /// Namespaced resource within a given namespace
    pub fn namespaced<K: NamespaceScopedResource>(ns: &str) -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            scope: ResourceScope::Namespace(ns.to_string()),
        }
    }
}

// -------------------------------------------------------

impl Resource {
    pub(crate) fn make_url(&self) -> String {
        let n = if let ResourceScope::Namespace(ns) = &self.scope {
            format!("namespaces/{}/", ns)
        } else {
            "".into()
        };
        format!(
            "/{group}/{api_version}/{namespaces}{resource}",
            group = if self.group.is_empty() { "api" } else { "apis" },
            api_version = self.api_version,
            namespaces = n,
            resource = to_plural(&self.kind.to_ascii_lowercase()),
        )
    }
}

/// Common query parameters used in watch/list/delete calls on collections
///
/// Constructed internally with a builder on Informer and Reflector,
/// but can be passed to the helper function of Resource.
#[derive(Default, Clone)]
pub struct ListParams {
    pub field_selector: Option<String>,
    pub include_uninitialized: bool,
    pub label_selector: Option<String>,
    pub timeout: Option<u32>,
}

impl ListParams {
    fn validate(&self) -> Result<()> {
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
    /// Supports '=', '==', and '!=', and can comma separate: key1=value1,key2=value2
    /// The server only supports a limited number of field queries per type.
    pub fn fields(mut self, field_selector: &str) -> Self {
        self.field_selector = Some(field_selector.to_string());
        self
    }

    /// Configure the selector to restrict the list of returned objects by their labels.
    ///
    /// Defaults to everything.
    /// Supports '=', '==', and '!=', and can comma separate: key1=value1,key2=value2
    pub fn labels(mut self, label_selector: &str) -> Self {
        self.label_selector = Some(label_selector.to_string());
        self
    }

    /// If called, partially initialized resources are included in watch/list responses.
    pub fn include_uninitialized(mut self) -> Self {
        self.include_uninitialized = true;
        self
    }
}

// TODO: WatchParams (same as ListParams but with extra resource_version + allow_watch_bookmarks)

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
    pub field_manager: Option<String>,
}

impl PatchParams {
    fn validate(&self) -> Result<()> {
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

/// Convenience methods found from API conventions
impl Resource {
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
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Watch a resource at a given version
    pub fn watch(&self, lp: &ListParams, ver: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        lp.validate()?;

        qp.append_pair("watch", "true");
        qp.append_pair("resourceVersion", ver);

        // https://github.com/kubernetes/kubernetes/issues/6513
        qp.append_pair("timeoutSeconds", &lp.timeout.unwrap_or(290).to_string());
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
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Get a single instance
    pub fn get(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name;
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Create an instance of a resource
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::post(urlstr);
        req.body(data).map_err(Error::HttpError)
    }

    /// Delete an instance of a resource
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "?";
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
        let req = http::Request::delete(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
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
        let req = http::Request::delete(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
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

        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch)
            .map_err(Error::HttpError)
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
        let req = http::Request::put(urlstr);
        req.body(data).map_err(Error::HttpError)
    }
}

/// Scale subresource
impl Resource {
    /// Get an instance of the scale subresource
    pub fn get_scale(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/scale";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Patch an instance of the scale subresource
    pub fn patch_scale(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let base_url = self.make_url() + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch)
            .map_err(Error::HttpError)
    }

    /// Replace an instance of the scale subresource
    pub fn replace_scale(
        &self,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr);
        req.body(data).map_err(Error::HttpError)
    }
}

/// Status subresource
impl Resource {
    /// Get an instance of the status subresource
    pub fn get_status(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/status";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Patch an instance of the status subresource
    pub fn patch_status(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let base_url = self.make_url() + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", pp.patch_strategy.to_string())
            .body(patch)
            .map_err(Error::HttpError)
    }

    /// Replace an instance of the status subresource
    pub fn replace_status(
        &self,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr);
        req.body(data).map_err(Error::HttpError)
    }
}

/// Extensive tests for Resource::<k8s_openapi::Resource impls>
///
/// Cheap sanity check to ensure type maps work as expected
/// Only uses Resource::create to check the general url format.
#[cfg(test)]
mod test {
    use crate::api::{PostParams, Resource};

    use k8s::{
        admissionregistration::v1beta1 as adregv1beta1,
        apps::v1 as appsv1,
        authorization::v1 as authv1,
        autoscaling::v1 as autoscalingv1,
        batch::v1beta1 as batchv1beta1,
        core::v1 as corev1,
        extensions::v1beta1 as extsv1beta1,
        networking::{v1 as networkingv1, v1beta1 as networkingv1beta1},
        rbac::v1 as rbacv1,
        storage::v1 as storagev1,
    };
    use k8s_openapi::api as k8s;
    // use k8s::batch::v1 as batchv1;

    // NB: stable requires >= 1.17
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiextsv1;

    // TODO: fixturize these tests
    // these are sanity tests for macros that create the Resource::v1Ctors
    #[test]
    fn api_url_secret() {
        use k8s_openapi::Resource as ResourceTrait;
        let r = Resource::namespaced::<corev1::Secret>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        println!("trait is: {:?}", corev1::Secret::GROUP);
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/secrets?");
    }

    #[test]
    fn api_url_rs() {
        let r = Resource::namespaced::<appsv1::ReplicaSet>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
    }
    #[test]
    fn api_url_role() {
        let r = Resource::namespaced::<rbacv1::Role>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/rbac.authorization.k8s.io/v1/namespaces/ns/roles?"
        );
    }

    #[test]
    fn api_url_cj() {
        let r = Resource::namespaced::<batchv1beta1::CronJob>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/batch/v1beta1/namespaces/ns/cronjobs?");
    }
    #[test]
    fn api_url_hpa() {
        let r = Resource::namespaced::<autoscalingv1::HorizontalPodAutoscaler>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/autoscaling/v1/namespaces/ns/horizontalpodautoscalers?"
        );
    }

    #[test]
    fn api_url_np() {
        let r = Resource::namespaced::<networkingv1::NetworkPolicy>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1/namespaces/ns/networkpolicies?"
        );
    }
    #[test]
    fn api_url_ingress() {
        let r = Resource::namespaced::<extsv1beta1::Ingress>("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/extensions/v1beta1/namespaces/ns/ingresses?");
    }

    #[test]
    fn api_url_vattach() {
        let r = Resource::cluster::<storagev1::VolumeAttachment>();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }

    #[test]
    fn api_url_admission() {
        let r = Resource::cluster::<adregv1beta1::ValidatingWebhookConfiguration>();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1beta1/validatingwebhookconfigurations?"
        );
    }

    #[test]
    fn api_auth_selfreview() {
        let r = Resource::cluster::<authv1::SelfSubjectRulesReview>();
        assert_eq!(r.group, "authorization.k8s.io");
        assert_eq!(r.kind, "SelfSubjectRulesReview");

        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/authorization.k8s.io/v1/selfsubjectrulesreviews?"
        );
    }

    #[test]
    fn api_apiextsv1_crd() {
        let r = Resource::cluster::<apiextsv1::CustomResourceDefinition>();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1/customresourcedefinitions?"
        );
    }

    /// -----------------------------------------------------------------
    /// Tests that the misc mappings are also sensible
    use crate::api::{DeleteParams, ListParams, PatchParams, PatchStrategy};
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiextsv1beta1;

    #[test]
    fn list_path() {
        let r = Resource::namespaced::<appsv1::Deployment>("ns");
        let gp = ListParams::default();
        let req = r.list(&gp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
    }
    #[test]
    fn watch_path() {
        let r = Resource::namespaced::<corev1::Pod>("ns");
        let gp = ListParams::default();
        let req = r.watch(&gp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&resourceVersion=0&timeoutSeconds=290"
        );
    }
    #[test]
    fn replace_path() {
        let r = Resource::all::<appsv1::DaemonSet>();
        let pp = PostParams {
            dry_run: true,
            ..Default::default()
        };
        let req = r.replace("myds", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/daemonsets/myds?&dryRun=All");
    }

    #[test]
    fn delete_path() {
        let r = Resource::namespaced::<appsv1::ReplicaSet>("ns");
        let dp = DeleteParams::default();
        let req = r.delete("myrs", &dp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn delete_collection_path() {
        let r = Resource::namespaced::<appsv1::ReplicaSet>("ns");
        let lp = ListParams::default();
        let req = r.delete_collection(&lp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn namespace_path() {
        let r = Resource::cluster::<corev1::Namespace>();
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
        assert!(
            patch_strategy_apply_true.validate().is_err(),
            "Merge strategy shouldn't be valid if `force` set to true"
        );
    }

    // subresources with weird version accuracy
    #[test]
    fn patch_status_path() {
        let r = Resource::cluster::<corev1::Node>();
        let pp = PatchParams::default();
        let req = r.patch_status("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
        assert_eq!(
            req.headers().get("Content-Type").unwrap().to_str().unwrap(),
            format!("{}", PatchStrategy::Merge)
        );
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_status_path() {
        let r = Resource::cluster::<corev1::Node>();
        let pp = PostParams::default();
        let req = r.replace_status("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
        assert_eq!(req.method(), "PUT");
    }

    #[test]
    fn create_ingress() {
        // NB: Ingress exists in extensions AND networking
        let r = Resource::namespaced::<networkingv1beta1::Ingress>("ns");
        let pp = PostParams::default();
        let req = r.create(&pp, vec![]).unwrap();

        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1beta1/namespaces/ns/ingresses?"
        );
        let patch_params = PatchParams::default();
        let req = r.patch("baz", &patch_params, vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1beta1/namespaces/ns/ingresses/baz?"
        );
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn replace_status() {
        let r = Resource::cluster::<apiextsv1beta1::CustomResourceDefinition>();
        let pp = PostParams::default();
        let req = r.replace_status("mycrd.domain.io", &pp, vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1beta1/customresourcedefinitions/mycrd.domain.io/status?"
        );
    }
    #[test]
    fn get_scale_path() {
        let r = Resource::cluster::<corev1::Node>();
        let req = r.get_scale("mynode").unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale");
        assert_eq!(req.method(), "GET");
    }
    #[test]
    fn patch_scale_path() {
        let r = Resource::cluster::<corev1::Node>();
        let pp = PatchParams::default();
        let req = r.patch_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_scale_path() {
        let r = Resource::cluster::<corev1::Node>();
        let pp = PostParams::default();
        let req = r.replace_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PUT");
    }

/*    #[test]
    #[should_panic] - compile fails now!
    fn all_resources_not_namespaceable() {
        Resource::namespaced::<corev1::Node>("ns");
    }*/
}
