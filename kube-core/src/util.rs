//! Utils and helpers

use crate::{
    params::{Patch, PatchParams},
    request, Request,
};
use chrono::Utc;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};

/// Restartable Resource marker trait
pub trait Restart {}

impl Restart for Deployment {}
impl Restart for DaemonSet {}
impl Restart for StatefulSet {}
impl Restart for ReplicaSet {}

impl Request {
    /// Restart a resource
    pub fn restart(&self, name: &str) -> Result<http::Request<Vec<u8>>, request::Error> {
        let patch = serde_json::json!({
          "spec": {
            "template": {
              "metadata": {
                "annotations": {
                  "kube.kubernetes.io/restartedAt": Utc::now().to_rfc3339()
                }
              }
            }
          }
        });

        let pparams = PatchParams::default();
        self.patch(name, &pparams, &Patch::Merge(patch))
    }
}

impl Request {
    /// Cordon a resource
    pub fn cordon(&self, name: &str) -> Result<http::Request<Vec<u8>>, request::Error> {
        self.set_unschedulable(name, true)
    }

    /// Uncordon a resource
    pub fn uncordon(&self, name: &str) -> Result<http::Request<Vec<u8>>, request::Error> {
        self.set_unschedulable(name, false)
    }

    fn set_unschedulable(
        &self,
        node_name: &str,
        value: bool,
    ) -> Result<http::Request<Vec<u8>>, request::Error> {
        self.patch(
            node_name,
            &PatchParams::default(),
            &Patch::Strategic(serde_json::json!({ "spec": { "unschedulable": value } })),
        )
    }
}

#[cfg(test)]
mod test {
    use crate::{params::Patch, request::Request, resource::Resource};

    #[test]
    fn restart_patch_is_correct() {
        use k8s_openapi::api::apps::v1 as appsv1;

        let url = appsv1::Deployment::url_path(&(), Some("ns"));
        let req = Request::new(url).restart("mydeploy").unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/deployments/mydeploy?");
        assert_eq!(req.method(), "PATCH");
        assert_eq!(
            req.headers().get("Content-Type").unwrap().to_str().unwrap(),
            Patch::Merge(()).content_type()
        );
    }

    #[test]
    fn cordon_patch_is_correct() {
        use k8s_openapi::api::core::v1::Node;

        let url = Node::url_path(&(), Some("ns"));
        let req = Request::new(url).cordon("mynode").unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/nodes/mynode?");
        assert_eq!(req.method(), "PATCH");
        assert_eq!(
            req.headers().get("Content-Type").unwrap().to_str().unwrap(),
            Patch::Strategic(()).content_type()
        );
    }
}
