//! Utils and helpers

use crate::{
    params::{Patch, PatchParams},
    Request, Result,
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
    pub fn restart(&self, name: &str) -> Result<http::Request<Vec<u8>>> {
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
