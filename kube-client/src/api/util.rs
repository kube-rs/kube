use crate::{
    api::{Api, Resource},
    Error, Result,
};
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
