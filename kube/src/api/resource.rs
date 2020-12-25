use super::params::{DeleteParams, ListParams, PatchParams, PostParams};
use crate::{api::DynamicResource, Error, Result};
use inflector::string::pluralize::to_plural;

/// A Kubernetes resource that can be accessed through the API
#[derive(Clone, Debug)]
pub struct Resource {
    /// The API version of the resource.
    ///
    /// This is a composite of `Resource::GROUP` and `Resource::VERSION`
    /// (eg "apiextensions.k8s.io/v1beta1")
    /// or just the version for resources without a group (eg "v1").
    /// This is the string used in the `apiVersion` field of the resource's serialized form.
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
}

impl Resource {
    /// Cluster level resources, or resources viewed across all namespaces
    pub fn all<K: k8s_openapi::Resource>() -> Self {
        Self {
            api_version: K::API_VERSION.to_string(),
            kind: K::KIND.to_string(),
            group: K::GROUP.to_string(),
            version: K::VERSION.to_string(),
            namespace: None,
        }
    }

    /// Namespaced resource within a given namespace
    pub fn namespaced<K: k8s_openapi::Resource>(ns: &str) -> Self {
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
        }
    }

    /// Manually configured resource or custom resource
    ///
    /// This is the only entrypoint to `Resource` that bypasses [`k8s_openapi`] entirely.
    /// If you need a `CustomResource` consider using `kube-derive` for its
    /// `#[derive(CustomResource)]` proc-macro.
    pub fn dynamic(kind: &str) -> DynamicResource {
        DynamicResource::new(kind)
    }
}

// -------------------------------------------------------

impl Resource {
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

/// Convenience methods found from API conventions
impl Resource {
    /// List a collection of a resource
    pub fn list(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }
        if let Some(limit) = &lp.limit {
            qp.append_pair("limit", &limit.to_string());
        }
        if let Some(continue_token) = &lp.continue_token {
            qp.append_pair("continue", continue_token);
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
        if lp.limit.is_some() {
            return Err(Error::RequestValidation(
                "ListParams::limit cannot be used with a watch.".into(),
            ));
        }
        if lp.continue_token.is_some() {
            return Err(Error::RequestValidation(
                "ListParams::continue_token cannot be used with a watch.".into(),
            ));
        }

        qp.append_pair("watch", "true");
        qp.append_pair("resourceVersion", ver);

        // https://github.com/kubernetes/kubernetes/issues/6513
        qp.append_pair("timeoutSeconds", &lp.timeout.unwrap_or(290).to_string());
        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }
        if lp.allow_bookmarks {
            qp.append_pair("allowWatchBookmarks", "true");
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
        pp.validate()?;
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
        let urlstr = qp.finish();
        let body = serde_json::to_vec(&dp)?;
        let req = http::Request::delete(urlstr);
        req.body(body).map_err(Error::HttpError)
    }

    /// Delete a collection of a resource
    pub fn delete_collection(&self, dp: &DeleteParams, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url() + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", &labels);
        }
        let urlstr = qp.finish();
        let body = serde_json::to_vec(&dp)?;
        let req = http::Request::delete(urlstr);
        req.body(body).map_err(Error::HttpError)
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
    /// Requires `metadata.resourceVersion` set in data
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
        let r = Resource::all::<storagev1::VolumeAttachment>();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }

    #[test]
    fn api_url_admission() {
        let r = Resource::all::<adregv1beta1::ValidatingWebhookConfiguration>();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1beta1/validatingwebhookconfigurations?"
        );
    }

    #[test]
    fn api_auth_selfreview() {
        let r = Resource::all::<authv1::SelfSubjectRulesReview>();
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
        let r = Resource::all::<apiextsv1::CustomResourceDefinition>();
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
        let dp = DeleteParams::default();
        let req = r.delete_collection(&dp, &lp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn namespace_path() {
        let r = Resource::all::<corev1::Namespace>();
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
        let r = Resource::all::<corev1::Node>();
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
        let r = Resource::all::<corev1::Node>();
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
        let r = Resource::all::<apiextsv1beta1::CustomResourceDefinition>();
        let pp = PostParams::default();
        let req = r.replace_status("mycrd.domain.io", &pp, vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1beta1/customresourcedefinitions/mycrd.domain.io/status?"
        );
    }
    #[test]
    fn get_scale_path() {
        let r = Resource::all::<corev1::Node>();
        let req = r.get_scale("mynode").unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale");
        assert_eq!(req.method(), "GET");
    }
    #[test]
    fn patch_scale_path() {
        let r = Resource::all::<corev1::Node>();
        let pp = PatchParams::default();
        let req = r.patch_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_scale_path() {
        let r = Resource::all::<corev1::Node>();
        let pp = PostParams::default();
        let req = r.replace_scale("mynode", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PUT");
    }

    #[test]
    #[should_panic]
    fn all_resources_not_namespaceable() {
        Resource::namespaced::<corev1::Node>("ns");
    }
}
