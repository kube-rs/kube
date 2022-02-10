use crate::{api::Api, Error, Result};
use k8s_openapi::api::certificates::v1::CertificateSigningRequest;
use kube_core::params::{Patch, PatchParams};


impl Api<CertificateSigningRequest> {
    /// Partially update approval of the specified CertificateSigningRequest.
    pub async fn patch_approval<P: serde::Serialize>(
        &self,
        name: &str,
        pp: &PatchParams,
        patch: &Patch<P>,
    ) -> Result<CertificateSigningRequest> {
        let mut req = self
            .request
            .patch_subresource("approval", name, pp, patch)
            .map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("approval");
        self.client.request::<CertificateSigningRequest>(req).await
    }

    /// Get the CertificateSigningRequest. May differ from get(name)
    pub async fn get_approval(&self, name: &str) -> Result<CertificateSigningRequest> {
        self.get_subresource("approval", name).await
    }
}
