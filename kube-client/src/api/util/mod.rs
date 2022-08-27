use crate::{
    api::{Api, Resource},
    Error, Result,
};
use k8s_openapi::{
    api::{
        authentication::v1::TokenRequest,
        core::v1::{Node, ServiceAccount},
    },
    apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition,
};
use kube_core::{
    params::{ListParams, PatchParams, PostParams},
    util::Restart,
    ResourceExt,
};
use serde::{de::DeserializeOwned, Serialize};

k8s_openapi::k8s_if_ge_1_19! {
    mod csr;
}

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

impl Api<ServiceAccount> {
    /// Create a TokenRequest of a ServiceAccount
    pub async fn create_token_request(
        &self,
        name: &str,
        pp: &PostParams,
        token_request: &TokenRequest,
    ) -> Result<TokenRequest> {
        let bytes = serde_json::to_vec(token_request).map_err(Error::SerdeError)?;
        let mut req = self
            .request
            .create_subresource("token", name, pp, bytes)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("create_token_request");
        self.client.request::<TokenRequest>(req).await
    }
}

/// Updates stored versions of all objects so that is matches
/// stored version of the CustomResourceDefinition.
///
/// This function makes apiserver
/// migrate all its instances to the version, marked with `storage=true`, and deletes all other versions
/// from the `.status.storedVersions`.
/// # Guarantees
/// - This function never corrupts or loses data and does not affect API clients (assuming that conversion is set up properly)
/// - If this function failed (or was aborted), you can safely call it again.
/// - If this function succeeded, you can safely call it again (but you shouldn't - this is useless)
/// - While this function runs, you can call safely it again in parallel (but you shouldn't - this is useless)
/// - If this function succeeded, you may safely stop serving any other api versions
pub async fn migrate_resources<K: Resource + Clone + Serialize + DeserializeOwned + std::fmt::Debug>(
    api: Api<K>,
    crds_api: Api<CustomResourceDefinition>,
    dt: &K::DynamicType,
) -> Result<()> {
    // fetch crd instance in advance so that we can compare-and-set it later.
    let mut crd = crds_api.get(&K::crd_name(dt)).await?;
    let objects = api.list(&ListParams::default()).await?;
    for object in objects.items {
        // apply empty patch: this will trigger conversion to the new version.
        api.patch(
            &object.name_unchecked(),
            &PatchParams::default(),
            &kube_core::params::Patch::Merge(serde_json::json!({})),
        )
        .await?;
    }
    let mut stored_version = None;
    for version in &crd.spec.versions {
        if version.storage {
            stored_version = Some(version.name.clone());
        }
    }
    let stored_version = stored_version.expect("No version has storage=true");
    if let Some(s) = crd.status.as_mut() {
        s.stored_versions = Some(vec![stored_version]);
    }
    crds_api
        .replace_status(
            &K::crd_name(dt),
            &PostParams::default(),
            serde_json::to_vec(&crd).unwrap(),
        )
        .await?;

    Ok(())
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
    use k8s_openapi::api::{
        authentication::v1::{TokenRequest, TokenRequestSpec, TokenReview, TokenReviewSpec},
        core::v1::{Node, ServiceAccount},
    };
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

        let schedulables = ListParams::default().fields("spec.unschedulable==false");
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

    #[tokio::test]
    #[ignore] // requires a cluster
    async fn create_token_request() -> Result<(), Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;

        let serviceaccount_name = "fakesa";
        let serviceaccount_namespace = "default";
        let audiences = vec!["api".to_string()];

        let serviceaccounts: Api<ServiceAccount> = Api::namespaced(client.clone(), serviceaccount_namespace);
        let tokenreviews: Api<TokenReview> = Api::all(client);

        // Create ServiceAccount
        let fake_sa = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "ServiceAccount",
            "metadata": {
                "name": serviceaccount_name,
            },
        }))?;
        serviceaccounts.create(&PostParams::default(), &fake_sa).await?;

        // Create TokenRequest
        let tokenrequest = serviceaccounts
            .create_token_request(serviceaccount_name, &PostParams::default(), &TokenRequest {
                metadata: Default::default(),
                spec: TokenRequestSpec {
                    audiences: audiences.clone(),
                    bound_object_ref: None,
                    expiration_seconds: None,
                },
                status: None,
            })
            .await?;
        let token = tokenrequest.status.unwrap().token;
        assert!(!token.is_empty());

        // Check created token is valid with TokenReview
        let tokenreview = tokenreviews
            .create(&PostParams::default(), &TokenReview {
                metadata: Default::default(),
                spec: TokenReviewSpec {
                    audiences: Some(audiences.clone()),
                    token: Some(token),
                },
                status: None,
            })
            .await?;
        let tokenreviewstatus = tokenreview.status.unwrap();
        assert_eq!(tokenreviewstatus.audiences, Some(audiences));
        assert_eq!(tokenreviewstatus.authenticated, Some(true));
        assert_eq!(tokenreviewstatus.error, None);
        assert_eq!(
            tokenreviewstatus.user.unwrap().username,
            Some(format!(
                "system:serviceaccount:{}:{}",
                serviceaccount_namespace, serviceaccount_name
            ))
        );

        // Cleanup ServiceAccount
        serviceaccounts
            .delete(serviceaccount_name, &DeleteParams::default())
            .await?;

        Ok(())
    }
}
