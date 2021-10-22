use crate::{
    api::{Api, Resource},
    Result,
};
use kube_core::util::Restart;
use serde::de::DeserializeOwned;

impl<K> Api<K>
where
    K: Restart + Resource + DeserializeOwned,
{
    /// Trigger a restart of a Resource.
    pub async fn restart(&self, name: &str) -> Result<K> {
        let mut req = self.request.restart(name)?;
        req.extensions_mut().insert("restart");
        self.client.request::<K>(req).await
    }
}
