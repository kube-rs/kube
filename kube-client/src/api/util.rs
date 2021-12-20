use crate::{
    api::{Api, Resource},
    Error, Result,
};
use k8s_openapi::api::core::v1::Node;
use kube_core::util::Restart;
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

    /// Cordon a Node.
    pub async fn uncordon(&self, name: &str) -> Result<Node> {
        let mut req = self.request.uncordon(name).map_err(Error::BuildRequest)?;
        req.extensions_mut().insert("cordon");
        self.client.request::<Node>(req).await
    }
}
