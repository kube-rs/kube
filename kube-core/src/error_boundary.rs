//! Types for isolating deserialization failures. See [`DeserializeGuard`].

use std::borrow::Cow;

use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde::Deserialize;
use serde_value::DeserializerError;
use thiserror::Error;

use crate::{PartialObjectMeta, Resource};

/// A wrapper type for K that lets deserializing the parent object succeed, even if the K is invalid.
///
/// For example, this can be used to still access valid objects from an `Api::list` call or `watcher`.
// We can't implement Deserialize on Result<K, InvalidObject> directly, both because of the orphan rule and because
// it would conflict with serde's blanket impl on Result<K, E>, even if E isn't Deserialize.
#[derive(Debug, Clone)]
pub struct DeserializeGuard<K>(pub Result<K, InvalidObject>);

/// An object that failed to be deserialized by the [`DeserializeGuard`].
#[derive(Debug, Clone, Error)]
#[error("{error}")]
pub struct InvalidObject {
    // Should ideally be D::Error, but we don't know what type it has outside of Deserialize::deserialize()
    // It *could* be Box<std::error::Error>, but we don't know that it is Send+Sync
    /// The error message from deserializing the object.
    pub error: String,
    /// The metadata of the invalid object.
    pub metadata: ObjectMeta,
}

impl<'de, K> Deserialize<'de> for DeserializeGuard<K>
where
    K: Deserialize<'de>,
    // Not actually used, but we assume that K is a Kubernetes-style resource with a `metadata` section
    K: Resource,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Deserialize::deserialize consumes the deserializer, and we want to retry parsing as an ObjectMetaContainer
        // if the initial parse fails, so that we can still implement Resource for the error case
        let buffer = serde_value::Value::deserialize(deserializer)?;

        // FIXME: can we avoid cloning the whole object? metadata should be enough, and even then we could prune managedFields
        K::deserialize(buffer.clone())
            .map(Ok)
            .or_else(|err| {
                let PartialObjectMeta { metadata, .. } =
                    PartialObjectMeta::<K>::deserialize(buffer).map_err(DeserializerError::into_error)?;
                Ok(Err(InvalidObject {
                    error: err.to_string(),
                    metadata,
                }))
            })
            .map(DeserializeGuard)
    }
}

impl<K: Resource> Resource for DeserializeGuard<K> {
    type DynamicType = K::DynamicType;
    type Scope = K::Scope;

    fn kind(dt: &Self::DynamicType) -> Cow<str> {
        K::kind(dt)
    }

    fn group(dt: &Self::DynamicType) -> Cow<str> {
        K::group(dt)
    }

    fn version(dt: &Self::DynamicType) -> Cow<str> {
        K::version(dt)
    }

    fn plural(dt: &Self::DynamicType) -> Cow<str> {
        K::plural(dt)
    }

    fn meta(&self) -> &ObjectMeta {
        self.0.as_ref().map_or_else(|err| &err.metadata, K::meta)
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        self.0.as_mut().map_or_else(|err| &mut err.metadata, K::meta_mut)
    }
}

#[cfg(test)]
mod tests {
    use k8s_openapi::api::core::v1::{ConfigMap, Pod};
    use serde_json::json;

    use crate::{DeserializeGuard, Resource};

    #[test]
    fn should_parse_meta_of_invalid_objects() {
        let pod_error = serde_json::from_value::<DeserializeGuard<Pod>>(json!({
            "metadata": {
                "name": "the-name",
                "namespace": "the-namespace",
            },
            "spec": {
                "containers": "not-a-list",
            },
        }))
        .unwrap();
        assert_eq!(pod_error.meta().name.as_deref(), Some("the-name"));
        assert_eq!(pod_error.meta().namespace.as_deref(), Some("the-namespace"));
        pod_error.0.unwrap_err();
    }

    #[test]
    fn should_allow_valid_objects() {
        let configmap = serde_json::from_value::<DeserializeGuard<ConfigMap>>(json!({
            "metadata": {
                "name": "the-name",
                "namespace": "the-namespace",
            },
            "data": {
                "foo": "bar",
            },
        }))
        .unwrap();
        assert_eq!(configmap.meta().name.as_deref(), Some("the-name"));
        assert_eq!(configmap.meta().namespace.as_deref(), Some("the-namespace"));
        assert_eq!(
            configmap.0.unwrap().data,
            Some([("foo".to_string(), "bar".to_string())].into())
        )
    }

    #[test]
    fn should_catch_invalid_objects() {
        serde_json::from_value::<DeserializeGuard<Pod>>(json!({
            "spec": {
                "containers": "not-a-list"
            }
        }))
        .unwrap()
        .0
        .unwrap_err();
    }
}
