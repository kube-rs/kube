use serde::de::DeserializeOwned;
use reqwest::StatusCode;
use std::marker::PhantomData;

use crate::api::api::{
    Api,
    PostParams,
    DeleteParams,
    ListParams,
};
use crate::api::resource::{
    ObjectList, Object,
};
use crate::client::{
    APIClient,
};
use crate::{Error, Result};

/// A typed Api variant that does not expose request internals
///
/// The upsides of working with this rather than `Api` direct are:
/// - super easy interface (no figuring out return types)
/// - openapi types for free
///
/// But the downsides are:
/// - k8s-openapi dependency required (behind feature)
/// - openapi types are unnecessarily heavy on Option use
/// - memory intensive structs because they contain the full data
/// - no control over requests (opinionated)
#[derive(Clone)]
pub struct OpenApi<P, U> {
    /// The request creator object
    api: Api,
    /// The client to use (from this library)
    client: APIClient,
    /// sPec and statUs structs
    phantom: (PhantomData<P>, PhantomData<U>),
}

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1beta1::{
    CustomResourceDefinitionSpec as CrdSpec,
    CustomResourceDefinitionStatus as CrdStatus,
};

impl OpenApi<CrdSpec, CrdStatus> {
    pub fn v1beta1CustomResourceDefinition(client: APIClient) -> Self {
        Self {
            api: Api::v1beta1CustomResourceDefinition(),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}

/// CRDs still need user structs
#[allow(non_snake_case)]
impl<P, U> OpenApi<P, U> where
    P: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned + Default,
{
    pub fn customResource(client: APIClient, name: &str) -> Self {
        Self {
            api: Api::customResource(name),
            client,
            phantom: (PhantomData, PhantomData)
        }
    }
}


/// Expose same interface as Api for controlling scope/group/versions/ns
impl<P, U> OpenApi<P, U> {
    pub fn within(mut self, ns: &str) -> Self {
        self.api = self.api.within(ns);
        self
    }
    pub fn group(mut self, group: &str) -> Self {
        self.api = self.api.group(group);
        self
    }
    pub fn version(mut self, version: &str) -> Self {
        self.api = self.api.version(version);
        self
    }
}


//type ObjectWithStatus<P, U> = Result<(Object<P, U>, StatusCode)>;
//type ObjectListWithStatus<P, U> = Result<(ObjectList<Object<P, U>>, StatusCode)>

/// PUSH/PUT/POST/GET abstractions
impl<P, U> OpenApi<P, U> where
    P: Clone + DeserializeOwned,
    U: Clone + DeserializeOwned + Default,
{
    pub fn get(&self, name: &str) -> Result<(Object<P, U>, StatusCode)> {
        let req = self.api.get(name)?;
        self.client.request::<Object<P, U>>(req)
    }
    pub fn create(&self, pp: &PostParams, data: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
        let req = self.api.create(&pp, data)?;
        self.client.request::<Object<P, U>>(req)
    }
    pub fn delete(&self, name: &str, dp: &DeleteParams) -> Result<(Object<P, U>, StatusCode)> {
        let req = self.api.delete(name, &dp)?;
        self.client.request::<Object<P, U>>(req)
    }
    pub fn list(&self, lp: &ListParams) -> Result<(ObjectList<Object<P, U>>, StatusCode)> {
        let req = self.api.list(&lp)?;
        self.client.request::<ObjectList<Object<P, U>>>(req)
    }
    pub fn delete_collection(&self, lp: &ListParams) -> Result<(ObjectList<Object<P, U>>, StatusCode)> {
        let req = self.api.delete_collection(&lp)?;
        self.client.request::<ObjectList<Object<P, U>>>(req)
    }
    pub fn patch(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
        let req = self.api.patch(name, &pp, patch)?;
        self.client.request::<Object<P, U>>(req)
    }
    pub fn replace(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
        let req = self.api.replace(name, &pp, data)?;
        self.client.request::<Object<P, U>>(req)
    }


/*
    pub fn get_scale(&self, name: &str) -> Result<(Object<P, U>, StatusCode)> {
    }
    pub fn patch_scale(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
    }
    pub fn replace_scale(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
    }
    pub fn get_status(&self, name: &str) -> Result<(Object<P, U>, StatusCode)> {
    }
    pub fn patch_status(&self, name: &str, pp: &PostParams, patch: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
    }
    pub fn replace_status(&self, name: &str, pp: &PostParams, data: Vec<u8>) -> Result<(Object<P, U>, StatusCode)> {
    }
*/

}
