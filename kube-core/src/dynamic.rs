//! Contains types for using resource kinds not known at compile-time.
//!
//! For concrete usage see [examples prefixed with dynamic_](https://github.com/kube-rs/kube/tree/main/examples).
pub use crate::discovery::ApiResource;
use crate::{
    metadata::TypeMeta,
    resource::{DynamicResourceScope, Resource},
};

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use std::borrow::Cow;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to parse this DynamicObject into a Resource: {source}")]
/// Failed to parse `DynamicObject` into `Resource`
pub struct ParseDynamicObjectError {
    #[from]
    source: serde_json::Error,
}

/// A dynamic representation of a kubernetes object
///
/// This will work with any non-list type object.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct DynamicObject {
    /// The type fields, not always present
    #[serde(flatten, default)]
    pub types: Option<TypeMeta>,
    /// Object metadata
    #[serde(default)]
    pub metadata: ObjectMeta,

    /// All other keys
    #[serde(flatten)]
    pub data: serde_json::Value,
}

impl DynamicObject {
    /// Create a DynamicObject with minimal values set from ApiResource.
    #[must_use]
    pub fn new(name: &str, resource: &ApiResource) -> Self {
        Self {
            types: Some(TypeMeta {
                api_version: resource.api_version.to_string(),
                kind: resource.kind.to_string(),
            }),
            metadata: ObjectMeta {
                name: Some(name.to_string()),
                ..Default::default()
            },
            data: Default::default(),
        }
    }

    /// Attach dynamic data to a DynamicObject
    #[must_use]
    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    /// Attach a namespace to a DynamicObject
    #[must_use]
    pub fn within(mut self, ns: &str) -> Self {
        self.metadata.namespace = Some(ns.into());
        self
    }

    /// Attempt to convert this `DynamicObject` to a `Resource`
    pub fn try_parse<K: Resource + for<'a> serde::Deserialize<'a>>(
        self,
    ) -> Result<K, ParseDynamicObjectError> {
        Ok(serde_json::from_value(serde_json::to_value(self)?)?)
    }
}

impl Resource for DynamicObject {
    type DynamicType = ApiResource;
    type Scope = DynamicResourceScope;

    fn group(dt: &ApiResource) -> Cow<'_, str> {
        dt.group.as_str().into()
    }

    fn version(dt: &ApiResource) -> Cow<'_, str> {
        dt.version.as_str().into()
    }

    fn kind(dt: &ApiResource) -> Cow<'_, str> {
        dt.kind.as_str().into()
    }

    fn api_version(dt: &ApiResource) -> Cow<'_, str> {
        dt.api_version.as_str().into()
    }

    fn plural(dt: &ApiResource) -> Cow<'_, str> {
        dt.plural.as_str().into()
    }

    fn meta(&self) -> &ObjectMeta {
        &self.metadata
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        &mut self.metadata
    }
}

#[cfg(test)]
mod test {
    use crate::{
        dynamic::{ApiResource, DynamicObject},
        gvk::GroupVersionKind,
        params::{Patch, PatchParams, PostParams},
        request::Request,
        resource::Resource,
    };
    use k8s_openapi::api::core::v1::Pod;

    #[test]
    fn raw_custom_resource() {
        let gvk = GroupVersionKind::gvk("clux.dev", "v1", "Foo");
        let res = ApiResource::from_gvk(&gvk);
        let url = DynamicObject::url_path(&res, Some("myns"));

        let pp = PostParams::default();
        let req = Request::new(&url).create(&pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos?");
        let patch_params = PatchParams::default();
        let req = Request::new(url)
            .patch("baz", &patch_params, &Patch::Merge(()))
            .unwrap();
        assert_eq!(req.uri(), "/apis/clux.dev/v1/namespaces/myns/foos/baz?");
        assert_eq!(req.method(), "PATCH");
    }

    #[test]
    fn raw_resource_in_default_group() {
        let gvk = GroupVersionKind::gvk("", "v1", "Service");
        let api_resource = ApiResource::from_gvk(&gvk);
        let url = DynamicObject::url_path(&api_resource, None);
        let pp = PostParams::default();
        let req = Request::new(url).create(&pp, vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/services?");
    }

    #[test]
    fn can_parse_dynamic_object_into_pod() -> Result<(), serde_json::Error> {
        let original_pod: Pod = serde_json::from_value(serde_json::json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "example" },
            "spec": {
                "containers": [{
                    "name": "example",
                    "image": "alpine",
                    // Do nothing
                    "command": ["tail", "-f", "/dev/null"],
                }],
            }
        }))?;
        let dynamic_pod: DynamicObject = serde_json::from_str(&serde_json::to_string(&original_pod)?)?;
        let parsed_pod: Pod = dynamic_pod.try_parse().unwrap();

        assert_eq!(parsed_pod, original_pod);

        Ok(())
    }
}
