use crate::{
    api::{Api, Resource},
    Error, Result,
};
use k8s_openapi::api::{
    certificates::v1::{
        CertificateSigningRequest, CertificateSigningRequestCondition, CertificateSigningRequestStatus,
    },
    core::v1::Node,
};
use kube_core::{
    params::{Patch, PatchParams},
    util::Restart,
};
use serde::de::DeserializeOwned;

impl<K> Api<K>
where
    K: Restart + Resource + DeserializeOwned,
{
    /// Trigger a restart of a Resource.
    pub async fn restart(&self, name: &str) -> Result<K> {
        let mut req = self.request.restart(name).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("restart");
        self.client.request::<K>(req).await
    }
}

impl Api<Node> {
    /// Cordon a Node.
    pub async fn cordon(&self, name: &str) -> Result<Node> {
        let mut req = self.request.cordon(name).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("cordon");
        self.client.request::<Node>(req).await
    }

    /// Uncordon a Node.
    pub async fn uncordon(&self, name: &str) -> Result<Node> {
        let mut req = self.request.uncordon(name).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("cordon");
        self.client.request::<Node>(req).await
    }
}

type CSRStatusType = &'static str;
static APPROVED: CSRStatusType = "Approved";
static DENIED: CSRStatusType = "Denied";

impl Api<CertificateSigningRequest> {
    /// Approve the specified CertificateSigningRequest.
    pub async fn approve(&self, name: &str, pp: &PatchParams) -> Result<CertificateSigningRequest> {
        self.patch_approval(name, pp, &APPROVED).await
    }

    /// Deny the specified CertificateSigningRequest.
    pub async fn deny(&self, name: &str, pp: &PatchParams) -> Result<CertificateSigningRequest> {
        self.patch_approval(name, pp, &DENIED).await
    }

    /// Partially update approval of the specified CertificateSigningRequest.
    async fn patch_approval(
        &self,
        name: &str,
        pp: &PatchParams,
        approval_type: &CSRStatusType,
    ) -> Result<CertificateSigningRequest> {
        let patch = serde_json::json!({"status": CertificateSigningRequestStatus {
            certificate:None,
            conditions: Some(vec![CertificateSigningRequestCondition {
                type_: approval_type.to_string(),
                last_update_time: None,
                last_transition_time: None,
                message: Some(format!("{} {}", approval_type, "by kube-rs client")),
                reason: Some("kube-rsClient".to_string()),
                status: "True".to_string()
            }])
        }});

        let mut req = self
            .request
            .patch_subresource("approval", name, pp, &Patch::Strategic(&patch))
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("approval");
        self.client.request::<CertificateSigningRequest>(req).await
    }
}

// Tests that require a cluster and the complete feature set
// Can be run with `cargo test -p kube-client --lib -- --ignored`
#[cfg(test)]
#[cfg(all(feature = "client"))]
mod test {
    use crate::{
        api::{Api, DeleteParams, ListParams, PostParams},
        Client,
    };
    use k8s_openapi::api::core::v1::Node;
    use serde_json::json;

    #[tokio::test]
    #[ignore] // needs kubeconfig
    async fn node_cordon_and_uncordon_works() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;

        let node_name = "fakenode";
        let fake_node = serde_json::from_value(json!({
        "apiVersion": "v1",
        "kind": "Node",
        "metadata": {
            "name": node_name,
            },
        }))?;

        let nodes: Api<Node> = Api::all(client.clone());
        nodes.create(&PostParams::default(), &fake_node).await?;

        let schedulables = ListParams::default().fields(&format!("spec.unschedulable==false"));
        let nodes_init = nodes.list(&schedulables).await?;
        let num_nodes_before_cordon = nodes_init.items.len();

        nodes.cordon(node_name).await?;
        let nodes_after_cordon = nodes.list(&schedulables).await?;
        assert_eq!(nodes_after_cordon.items.len(), num_nodes_before_cordon - 1);

        nodes.uncordon(node_name).await?;
        let nodes_after_uncordon = nodes.list(&schedulables).await?;
        assert_eq!(nodes_after_uncordon.items.len(), num_nodes_before_cordon);
        nodes.delete(node_name, &DeleteParams::default()).await?;
        Ok(())
    }
}
