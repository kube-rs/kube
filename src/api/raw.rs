use crate::{Error, Result};
use inflector::{cases::pascalcase::is_pascal_case, string::pluralize::to_plural};
use std::marker::PhantomData;

/// RawApi generation data
///
/// This data defines the urls used by kubernetes' APIs.
/// This struct is client agnostic, and can be passed to an Informer or a Reflector.
///
/// Can be used directly with a client.
/// When data is PUT/POST/PATCH'd this struct requires serialized raw bytes.
#[derive(Clone, Debug)]
pub struct RawApi<K> {
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

    /// The namespace if the resource resides (if namespaced)
    pub namespace: Option<String>,

    // hidden ref
    phantom: PhantomData<K>,
}

#[cfg(feature = "openapi")]
impl<K> From<K> for RawApi<K>
where
    K: k8s_openapi::Resource,
{
    fn from(_k: K) -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            namespace: None,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature = "openapi")]
impl<K> RawApi<K>
where
    K: k8s_openapi::Resource,
{
    pub fn global() -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            namespace: None,
            phantom: PhantomData,
        }
    }

    /// Set as namespaced resource within a specified namespace
    pub fn within(ns: &str) -> Self {
        match K::KIND {
            "Node" | "Namespace" | "ClusterRole" | "CustomResourceDefinition" => {
                panic!("{} is not a namespace scoped resource", K::KIND)
            }
            _ => {}
        }
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            namespace: Some(ns.to_string()),
            phantom: PhantomData,
        }
    }
}

/// A Resource trait equivalent for Custom Resource
///
/// This is the smallest amount of info we need to run the API against a CR
/// The version, and group must be set by the user:
///
/// ```
/// use kube::api::{CustomResource, RawApi};
/// struct Foo {
///     spec: FooSpec,
///     status: FooStatus,
/// };
/// let foos : RawApi<Foo> = CustomResource::new("Foo") // <.spec.kind>
///    .group("clux.dev") // <.spec.group>
///    .version("v1")
///    .build()
///    .into();
/// ```
pub struct CustomResource {
    kind: String,
    group: String,
    version: String,
    api_version: String,
    namespace: Option<String>,
}

impl CustomResource {
    /// Construct a CrBuilder
    pub fn new(kind: &str) -> CrBuilder {
        CrBuilder::new(kind)
    }
}

/// This impl makes CustomResource useable without k8s_openapi
impl<K> From<CustomResource> for RawApi<K> {
    fn from(c: CustomResource) -> Self {
        Self {
            api_version: c.api_version,
            kind: c.kind,
            group: c.group,
            version: c.version,
            namespace: c.namespace,
            phantom: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct CrBuilder {
    pub(crate) kind: String,
    pub(crate) version: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) namespace: Option<String>,
}
impl CrBuilder {
    /// Create a CrBuilder {
    fn new(kind: &str) -> Self {
        assert!(to_plural(kind) != kind); // no plural in kind
        assert!(is_pascal_case(&kind)); // PascalCase kind
        Self {
            kind: kind.into(),
            ..Default::default()
        }
    }

    /// Set the api group of a custom resource
    pub fn group(mut self, group: &str) -> Self {
        self.group = Some(group.to_string());
        self
    }

    /// Set the api version of a custom resource
    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(version.to_string());
        self
    }

    /// Set the namespace of a custom resource
    pub fn within(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    // Build a RawApi from Crd properties
    pub fn build(self) -> CustomResource {
        let version = self.version.expect("Crd must have a version");
        let group = self.group.expect("Crd must have a group");
        CustomResource {
            api_version: format!("{}/{}", group, version),
            kind: self.kind,
            version,
            group,
            namespace: self.namespace,
        }
    }
}

// -------------------------------------------------------

impl<K> RawApi<K> {
    pub(crate) fn make_url(&self) -> String {
        let n = if let Some(ns) = &self.namespace {
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
/// but can be passed to the helper function of RawApi.
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
impl<K> RawApi<K> {
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
impl<K> RawApi<K> {
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
impl<K> RawApi<K> {
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


// non-openapi tests
#[test]
fn create_custom_resource() {
    struct Foo {};
    let r: RawApi<Foo> = CustomResource::new("Foo")
        .group("clux.dev")
        .version("v1")
        .within("myns")
        .build()
        .into();
    let pp = PostParams::default();
    let req = r.create(&pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos?");
    let patch_params = PatchParams::default();
    let req = r.patch("baz", &patch_params, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos/baz?");
    assert_eq!(req.method(), "PATCH");
}


/// Extensive tests for RawApi::<k8s_openapi::Resource impls>
///
/// Cheap sanity check to ensure type maps work as expected
/// Only uses RawApi::create to check the general url format.
#[cfg(feature = "openapi")]
#[cfg(test)]
mod test {
    use crate::api::{PostParams, RawApi};

    use k8s::{apps::v1 as appsv1, core::v1 as corev1, rbac::v1 as rbacv1};
    use k8s_openapi::api as k8s;
    // use k8s::batch::v1 as batchv1;
    use k8s::{
        admissionregistration::v1beta1 as adregv1beta1,
        authorization::v1 as authv1,
        autoscaling::v1 as autoscalingv1,
        batch::v1beta1 as batchv1beta1,
        extensions::v1beta1 as extsv1beta1,
        networking::{v1 as networkingv1, v1beta1 as networkingv1beta1},
        storage::v1 as storagev1,
    };

    // NB: stable requires >= 1.17
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiextsv1;

    use k8s_openapi::Resource;
    // TODO: fixturize these tests
    // these are sanity tests for macros that create the RawApi::v1Ctors
    #[test]
    fn api_url_secret() {
        let r = RawApi::<corev1::Secret>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        println!("trait is: {:?}", corev1::Secret::GROUP);
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/secrets?");
    }

    #[test]
    fn api_url_rs() {
        let r = RawApi::<appsv1::ReplicaSet>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
    }
    #[test]
    fn api_url_role() {
        let r = RawApi::<rbacv1::Role>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/rbac.authorization.k8s.io/v1/namespaces/ns/roles?"
        );
    }

    #[test]
    fn api_url_cj() {
        let r = RawApi::<batchv1beta1::CronJob>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/batch/v1beta1/namespaces/ns/cronjobs?");
    }
    #[test]
    fn api_url_hpa() {
        let r = RawApi::<autoscalingv1::HorizontalPodAutoscaler>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/autoscaling/v1/namespaces/ns/horizontalpodautoscalers?"
        );
    }

    #[test]
    fn api_url_np() {
        let r = RawApi::<networkingv1::NetworkPolicy>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1/namespaces/ns/networkpolicies?"
        );
    }
    #[test]
    fn api_url_ingress() {
        let r = RawApi::<extsv1beta1::Ingress>::within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/extensions/v1beta1/namespaces/ns/ingresses?");
    }

    #[test]
    fn api_url_vattach() {
        let r = RawApi::<storagev1::VolumeAttachment>::global();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }

    #[test]
    fn api_url_admission() {
        let r = RawApi::<adregv1beta1::ValidatingWebhookConfiguration>::global();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1beta1/validatingwebhookconfigurations?"
        );
    }

    #[test]
    fn api_auth_selfreview() {
        let r = RawApi::<authv1::SelfSubjectRulesReview>::global();
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
        let r = RawApi::<apiextsv1::CustomResourceDefinition>::global();
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
        let r = RawApi::<appsv1::Deployment>::within("ns");
        let gp = ListParams::default();
        let req = r.list(&gp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
    }
    #[test]
    fn watch_path() {
        let r = RawApi::<corev1::Pod>::within("ns");
        let gp = ListParams::default();
        let req = r.watch(&gp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&resourceVersion=0&timeoutSeconds=290"
        );
    }
    #[test]
    fn replace_path() {
        let r = RawApi::<appsv1::DaemonSet>::global();
        let pp = PostParams {
            dry_run: true,
            ..Default::default()
        };
        let req = r.replace("myds", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/daemonsets/myds?&dryRun=All");
    }

    #[test]
    fn delete_path() {
        let r = RawApi::<appsv1::ReplicaSet>::within("ns");
        let dp = DeleteParams::default();
        let req = r.delete("myrs", &dp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn delete_collection_path() {
        let r = RawApi::<appsv1::ReplicaSet>::within("ns");
        let lp = ListParams::default();
        let req = r.delete_collection(&lp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn namespace_path() {
        let r = RawApi::<corev1::Namespace>::global();
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
        let r = RawApi::<corev1::Node>::global();
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
        let r = RawApi::<corev1::Node>::global();
        let pp = PostParams::default();
        let req = r.replace_status("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
        assert_eq!(req.method(), "PUT");
    }

    #[test]
    fn create_ingress() {
        // NB: Ingress exists in extensions AND networking
        let r = RawApi::<networkingv1beta1::Ingress>::within("ns");
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
        let r = RawApi::<apiextsv1beta1::CustomResourceDefinition>::global();
        let pp = PostParams::default();
        let req = r.replace_status("mycrd.domain.io", &pp, vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1beta1/customresourcedefinitions/mycrd.domain.io/status?"
        );
    }
    #[test]
    fn get_scale_path() {
        let r = RawApi::<corev1::Node>::global();
        let req = r.get_scale("mynode").unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale");
        assert_eq!(req.method(), "GET");
    }
    #[test]
    fn patch_scale_path() {
        let r = RawApi::<corev1::Node>::global();
        let pp = PatchParams::default();
        let req = r.patch_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_scale_path() {
        let r = RawApi::<corev1::Node>::global();
        let pp = PostParams::default();
        let req = r.replace_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PUT");
    }

    #[test]
    #[should_panic]
    fn global_resources_not_namespaceable() {
        RawApi::<corev1::Node>::within("ns");
    }
}
