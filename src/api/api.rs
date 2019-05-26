use crate::{Result, Error};


/// Resource representation from an API perspective
///
/// Used to construct requests from url conventions.
/// When data is PUT/POST/PATCH'd this struct requires raw bytes.
#[derive(Clone, Debug)]
pub struct Api {
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

    /// Version of status subresource (takes precedence)
    pub status_version: String,
    /// Version of scale subresource (takes precedence)
    pub scale_version: String,
}

impl Default for Api {
    fn default() -> Self {
        Self {
            resource: "pods".into(), // had to pick something here
            namespace: None,
            group: "".into(),
            version: "v1".into(),
            scale_version: "v1beta2".into(),
            status_version: "v1beta2".into(),
            prefix: "apis".into(), // seems most common
        }
    }
}

#[allow(non_snake_case)]
impl Api {
    pub fn within(mut self, ns: &str) -> Self {
        //match &self.resource {
        //    "nodes" | "namespaces" | "customresourcedefinitions" =>
        //        panic!("{} is not a namespaced resource", self.resource),
        //    _ => {},
        //}
        self.namespace = Some(ns.to_string());
        self
    }
    pub fn group(mut self, group: &str) -> Self {
        self.group = group.to_string();
        self
    }
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn v1Namespace() -> Self {
        Self {
            group: "".into(),
            resource: "namespaces".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }

    pub fn v1Deployment() -> Self {
        Self {
            group: "apps".into(),
            resource: "deployments".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    pub fn v1Pod() -> Self {
        Self {
            group: "".into(),
            resource: "pods".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }
    pub fn v1DaemonSet() -> Self {
        Self {
            group: "apps".into(),
            resource: "daemonsets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    pub fn v1ReplicaSet() -> Self {
        Self {
            group: "apps".into(),
            resource: "replicasets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    pub fn v1Node() -> Self {
        Self {
            group: "".into(),
            resource: "nodes".into(),
            prefix: "api".into(),
            ..Default::default()
        }
    }
    pub fn v1Statefulset() -> Self {
        Self {
            group: "apps".into(),
            resource: "statefulsets".into(),
            prefix: "apis".into(),
            ..Default::default()
        }
    }
    /// The definition of a customResource
    ///
    /// Its name MUST be in the format <.spec.name>.<.spec.group>.
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
    /// The version, and group must be set by the user.
    pub fn customResource(name: &str) -> Self {
        Self {
            resource: name.into(),
            ..Default::default()
        }
    }
}

// -------------------------------------------------------

enum UrlVersion {
    Default,
    Scale,
    Status,
}

impl Api {
    fn make_url(&self, r: UrlVersion) -> String {
        let pref = if self.prefix == "" { "".into() } else { format!("{}/", self.prefix) };
        let g = if self.group == "" { "".into() } else { format!("{}/", self.group) };
        let v = match r {
            UrlVersion::Scale => format!("{}/", self.scale_version),
            UrlVersion::Status => format!("{}/", self.status_version),
            UrlVersion::Default => format!("{}/", self.version)
        };
        let n = if let Some(ns) = &self.namespace { format!("namespaces/{}/", ns) } else { "".into() };

        format!("/{prefix}{group}{version}{namespaces}{resource}",
            prefix = pref,
            group = g,
            version = v,
            namespaces = n,
            resource = self.resource,
        )
    }
}

/// Common query parameters used in watch/list calls
///
/// Constructed internally with a builder on Informer and Reflector,
/// but can be passed to the helper function of Api.
#[derive(Default, Clone)]
pub struct GetParams {
    pub field_selector: Option<String>,
    pub include_uninitialized: bool,
    pub label_selector: Option<String>,
    pub timeout: Option<u32>
}

/// Common query parameters for put/post/patch calls
#[derive(Default, Clone)]
pub struct PostParams {
    pub dry_run: bool,
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

#[derive(Clone, Debug)]
pub enum PropagationPolicy {
    /// Orphan dependents
    Orphan,
    /// Allow the garbage collector to delete the dependents in the background
    Background,
    /// A cascading policy that deletes all dependents in the foreground
    Foreground,
}

impl Api {
    /// Create a list request to fully re-fetch the state
    pub fn list(&self, par: &GetParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        if let Some(fields) = &par.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if par.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        if let Some(labels) = &par.label_selector {
            qp.append_pair("labelSelector", &labels);
        }

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    /// Create a minimial list request to seed an initial resourceVersion
    pub(crate) fn list_zero_resource_entries(&self, par: &GetParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        qp.append_pair("limit", "1"); // can't have 0..
        if par.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        // rest of par doesn't matter here - we just need a resourceVersion
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    /// Create watch request for a Api at a given version
    pub(crate) fn watch(&self, par: &GetParams, ver: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);

        qp.append_pair("watch", "true");
        qp.append_pair("resourceVersion", ver);

        qp.append_pair("timeoutSeconds", &par.timeout.unwrap_or(10).to_string());
        if let Some(fields) = &par.field_selector {
            qp.append_pair("fieldSelector", &fields);
        }
        if par.include_uninitialized {
            qp.append_pair("includeUninitialized", "true");
        }
        if let Some(labels) = &par.label_selector {
            qp.append_pair("labelSelector", &labels);
        }

        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    /// Get a single instance
    pub fn get(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "/" + name;
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::post(urlstr);
        req.body(data).map_err(Error::from)
    }
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "/"+ name + "?";
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
        req.body(vec![]).map_err(Error::from)
    }

    pub fn patch(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "/" + name + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();

        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", "application/merge-patch+json")
            .body(patch).map_err(Error::from)
    }

    pub fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Default) + "/" + name + "?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        req.body(data).map_err(Error::from)
    }

    fn get_scale(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Scale) + "/" + name + "/scale";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    fn patch_scale(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Scale) + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", "application/merge-patch+json")
            .body(patch).map_err(Error::from)
    }
    fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Scale) + "/" + name + "/scale?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        req.body(data).map_err(Error::from)
    }

    fn get_status(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Status) + "/" + name + "/status";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        let urlstr = qp.finish();
        let mut req = http::Request::get(urlstr);
        req.body(vec![]).map_err(Error::from)
    }

    pub fn patch_status(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Status) + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        http::Request::patch(urlstr)
            .header("Accept", "application/json")
            .header("Content-Type", "application/merge-patch+json")
            .body(patch).map_err(Error::from)
    }

    pub fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<http::Request<Vec<u8>>> {
        let base_url = self.make_url(UrlVersion::Status) + "/" + name + "/status?";
        let mut qp = url::form_urlencoded::Serializer::new(base_url);
        if pp.dry_run {
            qp.append_pair("dryRun", "All");
        }
        let urlstr = qp.finish();
        let mut req = http::Request::put(urlstr);
        req.body(data).map_err(Error::from)
    }

}

#[test]
fn list_path(){
    let r = Api::v1Deployment().within("ns");
    let gp = GetParams::default();
    let req = r.list(&gp).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments");
}
#[test]
fn watch_path() {
    let r = Api::v1Pod().within("ns");
    let gp = GetParams::default();
    let req = r.watch(&gp, "0").unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces/ns/pods?&watch=true&resourceVersion=0&timeoutSeconds=10");
}
#[test]
fn replace_path(){
    let r = Api::v1DaemonSet();
    let pp = PostParams { dry_run: true, ..Default::default() };
    let req = r.replace("myds", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/daemonsets/myds?&dryRun=All");
}
#[test]
fn create_path() {
    let r = Api::v1ReplicaSet().within("ns");
    let pp = PostParams::default();
    let req = r.create(&pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
}
#[test]
fn delete_path() {
    let r = Api::v1ReplicaSet().within("ns");
    let dp = DeleteParams::default();
    let req = r.delete("myrs", &dp).unwrap();
    assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets/myrs");
    assert_eq!(req.method(), "DELETE")
}

#[test]
fn namespace_path() { // weird object compared to other v1
    let r = Api::v1Namespace();
    let gp = GetParams::default();
    let req = r.list(&gp).unwrap();
    assert_eq!(req.uri(), "/api/v1/namespaces")
}

// subresources with weird version accuracy
#[test]
fn patch_status_path(){
    let r = Api::v1Node();
    let pp = PostParams::default();
    let req = r.patch_status("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1beta2/nodes/mynode/status?");
    assert_eq!(req.method(), "PATCH");
}
#[test]
fn replace_status_path(){
    let r = Api::v1Node();
    let pp = PostParams::default();
    let req = r.replace_status("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1beta2/nodes/mynode/status?");
    assert_eq!(req.method(), "PUT");
}

#[test]
fn create_custom_resource() {
    let r = Api::customResource("foos")
        .group("clux.dev").version("v1")
        .within("myns");
    let pp = PostParams::default();
    let req = r.create(&pp, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/clux.dev/v1/namespaces/myns/foos?"
    );
    let req = r.patch("baz", &pp, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/clux.dev/v1/namespaces/myns/foos/baz?"
    );
}

#[test]
fn replace_status() {
    let r = Api::v1beta1CustomResourceDefinition();
    let pp = PostParams::default();
    let req = r.replace_status("mycrd.domain.io", &pp, vec![]).unwrap();
    assert_eq!(req.uri(),
        "/apis/apiextensions.k8s.io/v1beta2/customresourcedefinitions/mycrd.domain.io/status?"
    );
}
#[test]
fn patch_scale_path(){
    let r = Api::v1Node();
    let pp = PostParams::default();
    let req = r.patch_scale("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1beta2/nodes/mynode/scale?");
    assert_eq!(req.method(), "PATCH");
}
#[test]
fn replace_scale_path(){
    let r = Api::v1Node();
    let pp = PostParams::default();
    let req = r.replace_scale("mynode", &pp, vec![]).unwrap();
    assert_eq!(req.uri(), "/api/v1beta2/nodes/mynode/scale?");
    assert_eq!(req.method(), "PUT");
}
