//! Request builder type for arbitrary api types
use thiserror::Error;

use crate::params::GetParams;

use super::params::{DeleteParams, ListParams, Patch, PatchParams, PostParams, WatchParams};

pub(crate) const JSON_MIME: &str = "application/json";
/// Extended Accept Header
///
/// Requests a meta.k8s.io/v1 PartialObjectMetadata resource (efficiently
/// retrieves object metadata)
///
/// API Servers running Kubernetes v1.14 and below will retrieve the object and then
/// convert the metadata.
pub(crate) const JSON_METADATA_MIME: &str = "application/json;as=PartialObjectMetadata;g=meta.k8s.io;v=v1";

pub(crate) const JSON_METADATA_LIST_MIME: &str =
    "application/json;as=PartialObjectMetadataList;g=meta.k8s.io;v=v1";

/// Possible errors when building a request.
#[derive(Debug, Error)]
pub enum Error {
    /// Failed to build a request.
    #[error("failed to build request: {0}")]
    BuildRequest(#[source] http::Error),
    /// Failed to serialize body.
    #[error("failed to serialize body: {0}")]
    SerializeBody(#[source] serde_json::Error),
    /// Failed to validate request.
    #[error("failed to validate request: {0}")]
    Validation(String),
}

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
    pub fn list(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        lp.validate()?;
        lp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Watch a resource at a given version
    pub fn watch(&self, wp: &WatchParams, ver: &str) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        wp.validate()?;
        wp.populate_qp(&mut qp);
        qp.append_pair("resourceVersion", ver);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Get a single instance
    pub fn get(&self, name: &str, gp: &GetParams) -> Result<http::Request<Vec<u8>>, Error> {
        let urlstr = if let Some(rv) = &gp.resource_version {
            let target = format!("{}/{}?", self.url_path, name);
            form_urlencoded::Serializer::new(target)
                .append_pair("resourceVersion", rv)
                .finish()
        } else {
            let target = format!("{}/{}", self.url_path, name);
            form_urlencoded::Serializer::new(target).finish()
        };
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Create an instance of a resource
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>, Error> {
        pp.validate()?;
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::post(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::BuildRequest)
    }

    /// Delete an instance of a resource
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        let urlstr = qp.finish();
        let body = serde_json::to_vec(&dp).map_err(Error::SerializeBody)?;
        let req = http::Request::delete(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(body).map_err(Error::BuildRequest)
    }

    /// Delete a collection of a resource
    pub fn delete_collection(
        &self,
        dp: &DeleteParams,
        lp: &ListParams,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        if let Some(fields) = &lp.field_selector {
            qp.append_pair("fieldSelector", fields);
        }
        if let Some(labels) = &lp.label_selector {
            qp.append_pair("labelSelector", labels);
        }
        let urlstr = qp.finish();

        let data = if dp.is_default() {
            vec![] // default serialize needs to be empty body
        } else {
            serde_json::to_vec(&dp).map_err(Error::SerializeBody)?
        };

        let req = http::Request::delete(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::BuildRequest)
    }

    /// Patch an instance of a resource
    ///
    /// Requires a serialized merge-patch+json at the moment.
    pub fn patch<P: serde::Serialize>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        pp.validate(patch)?;
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header(http::header::ACCEPT, JSON_MIME)
            .header(http::header::CONTENT_TYPE, patch.content_type())
            .body(patch.serialize().map_err(Error::SerializeBody)?)
            .map_err(Error::BuildRequest)
    }

    /// Replace an instance of a resource
    ///
    /// Requires `metadata.resourceVersion` set in data
    pub fn replace(
        &self,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::BuildRequest)
    }
}

/// Subresources
impl Request {
    /// Get an instance of the subresource
    pub fn get_subresource(
        &self,
        subresource_name: &str,
        name: &str,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}/{}/{}", self.url_path, name, subresource_name);
        let mut qp = form_urlencoded::Serializer::new(target);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Create an instance of the subresource
    pub fn create_subresource(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}/{}/{}?", self.url_path, name, subresource_name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::post(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::BuildRequest)
    }

    /// Patch an instance of the subresource
    pub fn patch_subresource<P: serde::Serialize>(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        pp.validate(patch)?;
        let target = format!("{}/{}/{}?", self.url_path, name, subresource_name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header(http::header::ACCEPT, JSON_MIME)
            .header(http::header::CONTENT_TYPE, patch.content_type())
            .body(patch.serialize().map_err(Error::SerializeBody)?)
            .map_err(Error::BuildRequest)
    }

    /// Replace an instance of the subresource
    pub fn replace_subresource(
        &self,
        subresource_name: &str,
        name: &str,
        pp: &PostParams,
        data: Vec<u8>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}/{}/{}?", self.url_path, name, subresource_name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::put(urlstr).header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(data).map_err(Error::BuildRequest)
    }
}

/// Metadata-only request implementations
///
/// Requests set an extended Accept header compromised of JSON media type and
/// additional parameters that retrieve only necessary metadata from an object.
impl Request {
    /// Get a single metadata instance for a named resource
    pub fn get_metadata(&self, name: &str, gp: &GetParams) -> Result<http::Request<Vec<u8>>, Error> {
        let urlstr = if let Some(rv) = &gp.resource_version {
            let target = format!("{}/{}?", self.url_path, name);
            form_urlencoded::Serializer::new(target)
                .append_pair("resourceVersion", rv)
                .finish()
        } else {
            let target = format!("{}/{}", self.url_path, name);
            form_urlencoded::Serializer::new(target).finish()
        };
        let req = http::Request::get(urlstr)
            .header(http::header::ACCEPT, JSON_METADATA_MIME)
            .header(http::header::CONTENT_TYPE, JSON_MIME);
        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// List a collection of metadata of a resource
    pub fn list_metadata(&self, lp: &ListParams) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        lp.validate()?;
        lp.populate_qp(&mut qp);
        let urlstr = qp.finish();
        let req = http::Request::get(urlstr)
            .header(http::header::ACCEPT, JSON_METADATA_LIST_MIME)
            .header(http::header::CONTENT_TYPE, JSON_MIME);

        req.body(vec![]).map_err(Error::BuildRequest)
    }

    /// Watch metadata of a resource at a given version
    pub fn watch_metadata(&self, wp: &WatchParams, ver: &str) -> Result<http::Request<Vec<u8>>, Error> {
        let target = format!("{}?", self.url_path);
        let mut qp = form_urlencoded::Serializer::new(target);
        wp.validate()?;
        wp.populate_qp(&mut qp);
        qp.append_pair("resourceVersion", ver);

        let urlstr = qp.finish();
        http::Request::get(urlstr)
            .header(http::header::ACCEPT, JSON_METADATA_MIME)
            .header(http::header::CONTENT_TYPE, JSON_MIME)
            .body(vec![])
            .map_err(Error::BuildRequest)
    }

    /// Patch an instance of a resource and receive its metadata only
    ///
    /// Requires a serialized merge-patch+json at the moment
    pub fn patch_metadata<P: serde::Serialize>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<http::Request<Vec<u8>>, Error> {
        pp.validate(patch)?;
        let target = format!("{}/{}?", self.url_path, name);
        let mut qp = form_urlencoded::Serializer::new(target);
        pp.populate_qp(&mut qp);
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header(http::header::ACCEPT, JSON_METADATA_MIME)
            .header(http::header::CONTENT_TYPE, patch.content_type())
            .body(patch.serialize().map_err(Error::SerializeBody)?)
            .map_err(Error::BuildRequest)
    }
}

/// Extensive tests for Request of k8s_openapi::Resource structs
///
/// Cheap sanity check to ensure type maps work as expected
#[cfg(test)]
mod test {
    use crate::{
        params::{GetParams, PostParams, VersionMatch, WatchParams},
        request::Request,
        resource::Resource,
    };
    use http::header;
    use k8s::{
        admissionregistration::v1 as adregv1, apps::v1 as appsv1, authorization::v1 as authv1,
        autoscaling::v1 as autoscalingv1, batch::v1 as batchv1, core::v1 as corev1,
        networking::v1 as networkingv1, rbac::v1 as rbacv1, storage::v1 as storagev1,
    };
    use k8s_openapi::api as k8s;

    // NB: stable requires >= 1.17
    use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiextsv1;

    // TODO: fixturize these tests
    #[test]
    fn api_url_secret() {
        let url = corev1::Secret::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/secrets?");
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
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
        let url = batchv1::CronJob::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/batch/v1/namespaces/ns/cronjobs?");
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
        let url = networkingv1::Ingress::url_path(&(), Some("ns"));
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/networking.k8s.io/v1/namespaces/ns/ingresses?");
    }

    #[test]
    fn api_url_vattach() {
        let url = storagev1::VolumeAttachment::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }

    #[test]
    fn api_url_admission() {
        let url = adregv1::ValidatingWebhookConfiguration::url_path(&(), None);
        let req = Request::new(url).create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1/validatingwebhookconfigurations?"
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

    #[test]
    fn get_metadata_path() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let req = Request::new(url)
            .get_metadata("mydeploy", &GetParams::default())
            .unwrap();
        println!("{}", req.uri());
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments/mydeploy");
        assert_eq!(req.method(), "GET");
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
        assert_eq!(
            req.headers().get(header::ACCEPT).unwrap(),
            super::JSON_METADATA_MIME
        );
    }

    #[test]
    fn get_path_with_rv() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let req = Request::new(url).get("mydeploy", &GetParams::any()).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apps/v1/namespaces/ns/deployments/mydeploy?&resourceVersion=0"
        );
    }

    #[test]
    fn get_meta_path_with_rv() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let req = Request::new(url)
            .get_metadata("mydeploy", &GetParams::at("665"))
            .unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apps/v1/namespaces/ns/deployments/mydeploy?&resourceVersion=665"
        );

        assert_eq!(req.method(), "GET");
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
        assert_eq!(
            req.headers().get(header::ACCEPT).unwrap(),
            super::JSON_METADATA_MIME
        );
    }

    #[test]
    fn list_path() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let lp = ListParams::default();
        let req = Request::new(url).list(&lp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
    }
    #[test]
    fn list_metadata_path() {
        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let lp = ListParams::default().matching(VersionMatch::NotOlderThan).at("5");
        let req = Request::new(url).list_metadata(&lp).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apps/v1/namespaces/ns/deployments?&resourceVersion=5&resourceVersionMatch=NotOlderThan"
        );
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
        assert_eq!(
            req.headers().get(header::ACCEPT).unwrap(),
            super::JSON_METADATA_LIST_MIME
        );
    }
    #[test]
    fn watch_path() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let wp = WatchParams::default();
        let req = Request::new(url).watch(&wp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&timeoutSeconds=290&allowWatchBookmarks=true&resourceVersion=0"
        );
    }

    #[test]
    fn watch_streaming_list() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let wp = WatchParams::default().initial_events();
        let req = Request::new(url).watch(&wp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&timeoutSeconds=290&allowWatchBookmarks=true&sendInitialEvents=true&resourceVersionMatch=NotOlderThan&resourceVersion=0"
        );
    }

    #[test]
    fn watch_metadata_path() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let wp = WatchParams::default();
        let req = Request::new(url).watch_metadata(&wp, "0").unwrap();
        assert_eq!(
            req.uri(),
            "/api/v1/namespaces/ns/pods?&watch=true&timeoutSeconds=290&allowWatchBookmarks=true&resourceVersion=0"
        );
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
        assert_eq!(
            req.headers().get(header::ACCEPT).unwrap(),
            super::JSON_METADATA_MIME
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
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
    }

    #[test]
    fn delete_path() {
        let url = appsv1::ReplicaSet::url_path(&(), Some("ns"));
        let dp = DeleteParams::default();
        let req = Request::new(url).delete("myrs", &dp).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
        assert_eq!(req.method(), "DELETE");
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
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
        assert_eq!(req.method(), "DELETE");
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
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
    fn patch_pod_metadata() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let pp = PatchParams::default();
        let req = Request::new(url)
            .patch_metadata("mypod", &pp, &Patch::Merge(()))
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods/mypod?");
        assert_eq!(
            req.headers().get(header::CONTENT_TYPE).unwrap(),
            Patch::Merge(()).content_type()
        );
        assert_eq!(
            req.headers().get(header::ACCEPT).unwrap(),
            super::JSON_METADATA_MIME
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
        assert_eq!(req.headers().get(header::CONTENT_TYPE).unwrap(), super::JSON_MIME);
    }

    #[test]
    fn create_ingress() {
        // NB: Ingress exists in extensions AND networking
        let url = networkingv1::Ingress::url_path(&(), Some("ns"));
        let pp = PostParams::default();
        let req = Request::new(&url).create(&pp, vec![]).unwrap();

        assert_eq!(req.uri(), "/apis/networking.k8s.io/v1/namespaces/ns/ingresses?");
        let patch_params = PatchParams::default();
        let req = Request::new(url)
            .patch("baz", &patch_params, &Patch::Merge(()))
            .unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1/namespaces/ns/ingresses/baz?"
        );
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn replace_status() {
        let url = apiextsv1::CustomResourceDefinition::url_path(&(), None);
        let pp = PostParams::default();
        let req = Request::new(url)
            .replace_subresource("status", "mycrd.domain.io", &pp, vec![])
            .unwrap();
        assert_eq!(
            req.uri(),
            "/apis/apiextensions.k8s.io/v1/customresourcedefinitions/mycrd.domain.io/status?"
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

    #[test]
    fn create_subresource_path() {
        let url = corev1::ServiceAccount::url_path(&(), Some("ns"));
        let pp = PostParams::default();
        let data = vec![];
        let req = Request::new(url)
            .create_subresource("token", "sa", &pp, data)
            .unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/serviceaccounts/sa/token");
    }

    // TODO: reinstate if we get scoping in trait
    //#[test]
    //#[should_panic]
    //fn all_resources_not_namespaceable() {
    //    let _r = Request::<corev1::Node>::new(&(), Some("ns"));
    //}

    #[test]
    fn list_pods_from_cache() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default().match_any();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(
            req.uri().query().unwrap(),
            "&resourceVersion=0&resourceVersionMatch=NotOlderThan"
        );
    }

    #[test]
    fn list_most_recent_pods() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(
            req.uri().query().unwrap(),
            "" // No options are required
        );
    }

    #[test]
    fn list_invalid_resource_version_combination() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default().at("0").matching(VersionMatch::Exact);
        let err = Request::new(url).list(&gp).unwrap_err();
        assert!(format!("{err}").contains("non-zero resource_version is required when using an Exact match"));
    }

    #[test]
    fn list_paged_any_semantic() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default().limit(50).match_any();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(req.uri().query().unwrap(), "&limit=50");
    }

    #[test]
    fn list_paged_with_continue_any_semantic() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default().limit(50).continue_token("1234").match_any();
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(req.uri().query().unwrap(), "&limit=50&continue=1234");
    }

    #[test]
    fn list_paged_with_continue_starting_at() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default()
            .limit(50)
            .continue_token("1234")
            .at("9999")
            .matching(VersionMatch::Exact);
        let req = Request::new(url).list(&gp).unwrap();
        assert_eq!(req.uri().query().unwrap(), "&limit=50&continue=1234");
    }

    #[test]
    fn list_exact_match() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let gp = ListParams::default().at("500").matching(VersionMatch::Exact);
        let req = Request::new(url).list(&gp).unwrap();
        let query = req.uri().query().unwrap();
        assert_eq!(query, "&resourceVersion=500&resourceVersionMatch=Exact");
    }

    #[test]
    fn watch_params() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let wp = WatchParams::default()
            .disable_bookmarks()
            .fields("metadata.name=pod=1")
            .labels("app=web");
        let req = Request::new(url).watch(&wp, "0").unwrap();
        assert_eq!(
            req.uri().query().unwrap(),
            "&watch=true&timeoutSeconds=290&fieldSelector=metadata.name%3Dpod%3D1&labelSelector=app%3Dweb&resourceVersion=0"
        );
    }

    #[test]
    fn watch_timeout_error() {
        let url = corev1::Pod::url_path(&(), Some("ns"));
        let wp = WatchParams::default().timeout(100000);
        let err = Request::new(url).watch(&wp, "").unwrap_err();
        assert!(format!("{err}").contains("timeout must be < 295s"));
    }
}
