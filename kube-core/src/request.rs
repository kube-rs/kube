use super::params::{DeleteParams, ListParams, Patch, PatchParams, PostParams};
use crate::{Error, Result};

/// A Kubernetes request builder
///
/// Takes a base_path and supplies constructors for common operations
/// The extra operations all return `http::Request` objects.
#[derive(Debug, Clone)]
pub struct Request {
    /// The path component of a url
    pub url_path: String,
}

impl Request {
    /// New request with a resource's url path
    pub fn new<S: Into<String>>(url_path: S) -> Self {
        Self {
            url_path: url_path.into(),
        }
    }
}

// -------------------------------------------------------

/// Convenience methods found from API conventions
impl Request {
    /// List a collection of a resource
    pub fn list(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}?", self.url_path);
        let mut qp = url::form_urlencoded::Serializer::new(target);

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
        let target = format!("{}?", self.url_path);
        let mut qp = url::form_urlencoded::Serializer::new(target);
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
        if lp.bookmarks {
            qp.append_pair("allowWatchBookmarks", "true");
        }

        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Get a single instance
    pub fn get(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}", self.url_path, name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Create an instance of a resource
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        pp.validate()?;
        let target = format!("{}?", self.url_path);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::post(urlstr);
        req.body(data).map_err(Error::HttpError)
    }

    /// Delete an instance of a resource
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        let urlstr = qp.finish();
        let body = serde_json::to_vec(&dp)?;
        let req = http::Request::delete(urlstr);
        req.body(body).map_err(Error::HttpError)
    }

    /// Delete a collection of a resource
    pub fn delete_collection(&self, dp: &DeleteParams, lp: &ListParams) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}?", self.url_path);
        let mut qp = url::form_urlencoded::Serializer::new(target);
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
    pub fn patch<P: serde::Serialize>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<http::Request<Vec<u8>>> {
        pp.validate(patch)?;
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", patch.content_type())
            .body(patch.serialize()?)
            .map_err(Error::HttpError)
    }

    /// Replace an instance of a resource
    ///
    /// Requires `metadata.resourceVersion` set in data
    pub fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr);
        req.body(data).map_err(Error::HttpError)
    }
}

/// Subresources
impl Request {
    /// Get an instance of the subresource
    pub fn get_subresource(&self, subresource_name: &str, name: &str) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}/{}", self.url_path, name, subresource_name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::HttpError)
    }

    /// Patch an instance of the subresource
    pub fn patch_subresource<P: serde::Serialize>(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<http::Request<Vec<u8>>> {
        pp.validate(patch)?;
        let target = format!("{}/{}/{}?", self.url_path, name, subresource_name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", patch.content_type())
            .body(patch.serialize()?)
            .map_err(Error::HttpError)
    }

    /// Replace an instance of the subresource
    pub fn replace_subresource(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>> {
        let target = format!("{}/{}/{}?", self.url_path, name, subresource_name);
        let mut qp = url::form_urlencoded::Serializer::new(target);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr);
        req.body(data).map_err(Error::HttpError)
    }
}

/// Extensive tests for Request of k8s_openapi::Resource structs
///
/// Cheap sanity check to ensure type maps work as expected
#[cfg(test)]
mod test {
    use crate::{params::PostParams, request::Request, resource::Resource};
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
    #[test]
    fn api_url_secret() {
        let url = corev1::Secret::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/secrets?");
    }

    #[test]
    fn api_url_rs() {
        let url = appsv1::ReplicaSet::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
    }
    #[test]
    fn api_url_role() {
        let url = rbacv1::Role::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/rbac.authorization.k8s.io/v1/namespaces/ns/roles?"
        );
    }

    #[test]
    fn api_url_cj() {
        let url = batchv1beta1::CronJob::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/batch/v1beta1/namespaces/ns/cronjobs?");
    }
    #[test]
    fn api_url_hpa() {
        let url = autoscalingv1::HorizontalPodAutoscaler::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/autoscaling/v1/namespaces/ns/horizontalpodautoscalers?"
        );
    }

    #[test]
    fn api_url_np() {
        let url = networkingv1::NetworkPolicy::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1/namespaces/ns/networkpolicies?"
        );
    }
    #[test]
    fn api_url_ingress() {
        let url = extsv1beta1::Ingress::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/extensions/v1beta1/namespaces/ns/ingresses?");
    }

    #[test]
    fn api_url_vattach() {
        let url = storagev1::VolumeAttachment::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }

    #[test]
    fn api_url_admission() {
        let url = adregv1beta1::ValidatingWebhookConfiguration::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1beta1/validatingwebhookconfigurations?"
        );
    }

    #[test]
    fn api_auth_selfreview() {
        //assert_eq!(r.group, "authorization.k8s.io");
        //assert_eq!(r.kind, "SelfSubjectRulesReview");
        let url = authv1::SelfSubjectRulesReview::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/authorization.k8s.io/v1/selfsubjectrulesreviews?"
        );
    }

    #[test]
    fn api_apiextsv1_crd() {
        let url = apiextsv1::CustomResourceDefinition::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1/customresourcedefinitions?"
        );
    }

    /// -----------------------------------------------------------------
    /// Tests that the misc mappings are also sensible
    use crate::params::{DeleteParams, ListParams, Patch, PatchParams};
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1 as apiextsv1beta1;

    #[test]
    fn list_path() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let gp = ListParams::default();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
    }
    #[test]
    fn watch_path() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default();
        let req = Request::new(url).watch(&gp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&resourceVersion=0&timeoutSeconds=290&allowWatchBookmarks=true"
        );
    }
    #[test]
    fn replace_path() {
        let url = appsv1::DaemonSet::url_path(&(), None);
        let pp = PostParams {
            dry_run: true,
            ..Default::default()
        };
        let req = Request::new(url).replace("myds", &pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/daemonsets/myds?&dryRun=All");
    }

    #[test]
    fn delete_path() {
        let url = appsv1::ReplicaSet::url_path(&(), Some("ns"));
        let dp = DeleteParams::default();
        let req = Request::new(url).delete("myrs", &dp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn delete_collection_path() {
        let url = appsv1::ReplicaSet::url_path(&(), Some("ns"));
        let lp = ListParams::default().labels("app=myapp");
        let dp = DeleteParams::default();
        let req = Request::new(url).delete_collection(&dp, &lp).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apps/v1/namespaces/ns/replicasets?&labelSelector=app%3Dmyapp"
        );
        assert_eq!(req.method(), "DELETE")
    }

    #[test]
    fn namespace_path() {
        let url = corev1::Namespace::url_path(&(), None);
        let gp = ListParams::default();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces")
    }

    // subresources with weird version accuracy
    #[test]
    fn patch_status_path() {
        let url = corev1::Node::url_path(&(), None);
        let pp = PatchParams::default();
        let req = Request::new(url)
            .patch_subresource("status", "mynode", &pp, &Patch::Merge(()))
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
        assert_eq!(
            req.headers().get("Content-Type").unwrap().to_str().unwrap(),
            Patch::Merge(()).content_type()
        );
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_status_path() {
        let url = corev1::Node::url_path(&(), None);
        let pp = PostParams::default();
        let req = Request::new(url)
            .replace_subresource("status", "mynode", &pp, vec![])
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/status?");
        assert_eq!(req.method(), "PUT");
    }

    #[test]
    fn create_ingress() {
        // NB: Ingress exists in extensions AND networking
        let url = networkingv1beta1::Ingress::url_path(&(), Some("ns"));
        let pp = PostParams::default();
        let req = Request::new(&url).create(&pp, vec![]).unwrap();

        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1beta1/namespaces/ns/ingresses?"
        );
        let patch_params = PatchParams::default();
        let req = Request::new(url)
            .patch("baz", &patch_params, &Patch::Merge(()))
            .unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1beta1/namespaces/ns/ingresses/baz?"
        );
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn replace_status() {
        let url = apiextsv1beta1::CustomResourceDefinition::url_path(&(), None);
        let pp = PostParams::default();
        let req = Request::new(url)
            .replace_subresource("status", "mycrd.domain.io", &pp, vec![])
            .unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1beta1/customresourcedefinitions/mycrd.domain.io/status?"
        );
    }
    #[test]
    fn get_scale_path() {
        let url = corev1::Node::url_path(&(), None);
        let req = Request::new(url).get_subresource("scale", "mynode").unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale");
        assert_eq!(req.method(), "GET");
    }
    #[test]
    fn patch_scale_path() {
        let url = corev1::Node::url_path(&(), None);
        let pp = PatchParams::default();
        let req = Request::new(url)
            .patch_subresource("scale", "mynode", &pp, &Patch::Merge(()))
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PATCH");
    }
    #[test]
    fn replace_scale_path() {
        let url = corev1::Node::url_path(&(), None);
        let pp = PostParams::default();
        let req = Request::new(url)
            .replace_subresource("scale", "mynode", &pp, vec![])
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/nodes/mynode/scale?");
        assert_eq!(req.method(), "PUT");
    }

    // TODO: reinstate if we get scoping in trait
    //#[test]
    //#[should_panic]
    //fn all_resources_not_namespaceable() {
    //    let _r = Request::<corev1::Node>::new(&(), Some("ns"));
    //}
}
