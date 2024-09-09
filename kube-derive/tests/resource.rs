use k8s_openapi::{
    api::core::v1::{ConfigMap, Secret},
    ByteString,
};
use kube::api::ObjectMeta;
use kube_derive::Resource;

#[derive(Resource, Default)]
#[resource(inherit = "ConfigMap")]
struct TypedMap {
    metadata: ObjectMeta,
    data: Option<TypedData>,
}

#[derive(Default)]
struct TypedData {
    field: String,
}

#[derive(Resource, Default)]
#[resource(inherit = "Secret")]
struct TypedSecret {
    metadata: ObjectMeta,
    data: Option<TypedSecretData>,
}

#[derive(Default)]
struct TypedSecretData {
    field: ByteString,
}

#[cfg(test)]
mod tests {
    use kube::Resource;

    use crate::{TypedMap, TypedSecret};

    #[test]
    fn test_parse_config_map_default() {
        TypedMap::default();
        assert_eq!(TypedMap::kind(&()), "ConfigMap");
        assert_eq!(TypedMap::api_version(&()), "v1");
        assert_eq!(TypedMap::group(&()), "");
        assert_eq!(TypedMap::plural(&()), "configmaps");
    }

    #[test]
    fn test_parse_secret_default() {
        TypedSecret::default();
        assert_eq!(TypedSecret::kind(&()), "Secret");
        assert_eq!(TypedSecret::api_version(&()), "v1");
        assert_eq!(TypedSecret::group(&()), "");
        assert_eq!(TypedSecret::plural(&()), "secrets");
    }
}
