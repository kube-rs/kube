//! TODO

mod command;

pub use command::*;

use crate::api::DynamicObject;
use crate::client::Status;
use crate::command::command::KubeCommandVerb;
use crate::Api;
use crate::Client;
use crate::Error;
use either::Either;

/// TODO
pub struct Dispatcher {
    client: Client,
}

impl Dispatcher {
    /// TODO
    pub fn new(client: Client) -> Dispatcher {
        Dispatcher { client }
    }

    /// TODO
    pub async fn dispatch_command(
        &self,
        command: KubeCommand,
    ) -> Result<Either<DynamicObject, Status>, Error> {
        let api = match &command.namespace {
            None => Api::<DynamicObject>::all_with(self.client.clone(), &command.api_resource()),
            Some(namespace) => {
                Api::<DynamicObject>::namespaced_with(self.client.clone(), namespace, &command.api_resource())
            }
        };

        api.dispatch_command(command.verb).await
    }
}

impl Api<DynamicObject> {
    /// TODO
    pub async fn dispatch_command(
        &self,
        command: KubeCommandVerb,
    ) -> Result<Either<DynamicObject, Status>, Error> {
        match command {
            KubeCommandVerb::Create { object, params, .. } => {
                self.create(&params, &object).await.map(Either::Left)
            }
            KubeCommandVerb::Replace {
                name, object, params, ..
            } => self.replace(&name, &params, &object).await.map(Either::Left),
            KubeCommandVerb::ReplaceStatus {
                name, data, params, ..
            } => self.replace_status(&name, &params, data).await.map(Either::Left),
            KubeCommandVerb::Patch {
                name, patch, params, ..
            } => self.patch(&name, &params, &patch).await.map(Either::Left),
            KubeCommandVerb::PatchStatus {
                name, patch, params, ..
            } => self.patch_status(&name, &params, &patch).await.map(Either::Left),
            KubeCommandVerb::Delete { name, params, .. } => self.delete(&name, &params).await,
        }
    }
}
