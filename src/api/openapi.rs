use crate::api::RawApi;
#[cfg(feature = "openapi")]
use crate::{
    api::subresource::LoggingObject,
    api::{Api, NotUsed, Object},
    client::APIClient,
};
use inflector::string::pluralize::to_plural;
#[cfg(feature = "openapi")] use std::marker::PhantomData;

/// Bind typemeta properties for a k8s_openapi resource to RawApi
///
/// Constructs a RawApi::vxObjectName constructor with correct names, versions
macro_rules! k8s_obj {
    // 4 argument ver (allows customizing prefix)
    ( $name:expr, $version:expr, $prefix:expr, $group:expr) => {
        impl RawApi {
            paste::item! {
                #[allow(non_snake_case)]
                pub fn [<$version $name>]() -> Self {
                    Self {
                        prefix: $prefix.to_string(),
                        group: $group.to_string(),
                        resource: to_plural(&$name.to_ascii_lowercase()),
                        version: $version.to_string(),
                        ..Default::default()
                    }
                }
            }
        }
    };
    // 3 argument ver (for normal apis in the apis prefix)
    ( $name:expr, $version:expr, $group:expr) => {
        k8s_obj!($name, $version, "apis", $group);
    };
}

/// Special case k8s_obj invocation where we are in the special core empty prefix
macro_rules! k8s_core_obj {
    ( $name:expr, $version:expr, $group:expr) => {
        k8s_obj!($name, $version, "", $group);
    };
}

/// Bind a k8s_openapi resource struct to Api
///
/// Binds Api::vxObjectName to the RawApi
/// This uses the standard openapi path with consistent Spec and Status suffixed structs
macro_rules! k8s_ctor {
    ( $name:ident, $version:expr, $openapi:path) => {
        #[cfg(feature = "openapi")]
        paste::item! {
            impl Api<Object<
                $openapi::[<$version>]::[<$name Spec>],
                $openapi::[<$version>]::[<$name Status>]
                >> {
                #![allow(non_snake_case)]
                pub fn [<$version $name>](client: APIClient) -> Self {
                    Self {
                        api: RawApi::[<$version $name>](),
                        client,
                        phantom: PhantomData
                    }
                }
            }
        }
    };
}

/// Binds an arbitrary Object type to a verioned name on Api
///
/// Good for when there are api inconsistencies
#[cfg_attr(not(feature = "openapi"), allow(unused_macros))]
macro_rules! k8s_custom_ctor {
    ( $versioned_name:ident, $obj:ty) => {
        paste::item! {
            impl Api<$obj> {
                #![allow(non_snake_case)]
                pub fn [<$versioned_name>](client: APIClient) -> Self {
                    Self {
                        api: RawApi::[<$versioned_name>](),
                        client,
                        phantom: PhantomData
                    }
                }
            }
        }
    };
}


// api::apps
k8s_obj!("Deployment", "v1", "apps");
k8s_ctor!(Deployment, "v1", k8s_openapi::api::apps);
k8s_obj!("DaemonSet", "v1", "apps");
k8s_ctor!(DaemonSet, "v1", k8s_openapi::api::apps);
k8s_obj!("ReplicaSet", "v1", "apps");
k8s_ctor!(ReplicaSet, "v1", k8s_openapi::api::apps);
k8s_obj!("StatefulSet", "v1", "apps");
k8s_ctor!(StatefulSet, "v1", k8s_openapi::api::apps);


// api::authorization
k8s_obj!("SelfSubjectRulesReview", "v1", "authorization.k8s.io");
#[cfg(feature = "openapi")]
k8s_custom_ctor!(v1SelfSubjectRulesReview, Object<k8s_openapi::api::authorization::v1::SelfSubjectRulesReviewSpec, k8s_openapi::api::authorization::v1::SubjectRulesReviewStatus>);

// api::autoscaling
k8s_obj!("HorizontalPodAutoscaler", "v1", "autoscaling");
k8s_ctor!(HorizontalPodAutoscaler, "v1", k8s_openapi::api::autoscaling);

// api::admissionregistration
k8s_obj!(
    "ValidatingWebhookConfiguration",
    "v1beta1",
    "admissionregistration.k8s.io"
); // snowflake


// api::core
k8s_core_obj!("Pod", "v1", "api");
k8s_ctor!(Pod, "v1", k8s_openapi::api::core);
k8s_core_obj!("Node", "v1", "api");
k8s_ctor!(Node, "v1", k8s_openapi::api::core);
k8s_core_obj!("Service", "v1", "api");
k8s_ctor!(Service, "v1", k8s_openapi::api::core);
k8s_core_obj!("Namespace", "v1", "api");
k8s_ctor!(Namespace, "v1", k8s_openapi::api::core);
k8s_core_obj!("PersistentVolume", "v1", "api");
k8s_ctor!(PersistentVolume, "v1", k8s_openapi::api::core);
k8s_core_obj!("ResourceQuota", "v1", "api");
k8s_ctor!(ResourceQuota, "v1", k8s_openapi::api::core);
k8s_core_obj!("PersistentVolumeClaim", "v1", "api");
k8s_ctor!(PersistentVolumeClaim, "v1", k8s_openapi::api::core);
k8s_core_obj!("ReplicationController", "v1", "api");
k8s_ctor!(ReplicationController, "v1", k8s_openapi::api::core);

// snowflakes in api::core
k8s_core_obj!("Secret", "v1", "api");
k8s_core_obj!("Event", "v1", "api");
k8s_core_obj!("ConfigMap", "v1", "api");
k8s_core_obj!("ServiceAccount", "v1", "api");
k8s_core_obj!("Endpoints", "v1", "api"); // yup plural!

// subresources
#[cfg(feature = "openapi")]
impl LoggingObject for Object<k8s_openapi::api::core::v1::PodSpec, k8s_openapi::api::core::v1::PodStatus> {}
#[cfg(feature = "openapi")]
impl LoggingObject for Object<k8s_openapi::api::core::v1::PodSpec, NotUsed> {}


// api::batch
k8s_obj!("CronJob", "v1beta1", "batch");
k8s_ctor!(CronJob, "v1beta1", k8s_openapi::api::batch);
k8s_obj!("Job", "v1", "batch");
k8s_ctor!(Job, "v1", k8s_openapi::api::batch);


// api::extensions
k8s_obj!("Ingress", "v1beta1", "extensions");
k8s_ctor!(Ingress, "v1beta1", k8s_openapi::api::extensions);


// apiextensions_apiserver::pkg::apis::apiextensions
k8s_obj!("CustomResourceDefinition", "v1beta1", "apiextensions.k8s.io");
k8s_ctor!(
    CustomResourceDefinition,
    "v1beta1",
    k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions
);

#[cfg(feature = "openapi")]
k8s_openapi::k8s_if_ge_1_17! {
    k8s_obj!(
        "CustomResourceDefinition",
        "v1",
        "apiextensions.k8s.io"
    );
    k8s_ctor!(
        CustomResourceDefinition,
        "v1",
        k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions
    );
}


// api::rbac (snowflake objects in snowflake.rs)
k8s_obj!("Role", "v1", "rbac.authorization.k8s.io");
k8s_obj!("ClusterRole", "v1", "rbac.authorization.k8s.io");
k8s_obj!("RoleBinding", "v1", "rbac.authorization.k8s.io");


// api::storage::v1
k8s_obj!("VolumeAttachment", "v1", "storage.k8s.io");
k8s_ctor!(VolumeAttachment, "v1", k8s_openapi::api::storage);


// api::networking::v1
k8s_obj!("NetworkPolicy", "v1", "networking.k8s.io");
#[cfg(feature = "openapi")]
k8s_custom_ctor!(v1NetworkPolicy, Object<k8s_openapi::api::networking::v1::NetworkPolicySpec, NotUsed>); // no status


// Macro insanity needs some sanity here..
// There should be at least one test for each api group here to ensure no path typos
#[cfg(test)]
mod test {
    use crate::api::{PostParams, RawApi};
    // TODO: fixturize these tests
    // these are sanity tests for macros that create the RawApi::v1Ctors
    #[test]
    fn api_url_secret() {
        let r = RawApi::v1Secret().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/api/v1/namespaces/ns/secrets?");
    }
    #[test]
    fn api_url_rs() {
        let r = RawApi::v1ReplicaSet().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/apps/v1/namespaces/ns/replicasets?");
    }
    #[test]
    fn api_url_role() {
        let r = RawApi::v1Role().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/rbac.authorization.k8s.io/v1/namespaces/ns/roles?"
        );
    }
    #[test]
    fn api_url_cj() {
        let r = RawApi::v1beta1CronJob().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/batch/v1beta1/namespaces/ns/cronjobs?");
    }
    #[test]
    fn api_url_hpa() {
        let r = RawApi::v1HorizontalPodAutoscaler().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/autoscaling/v1/namespaces/ns/horizontalpodautoscalers?"
        );
    }
    #[test]
    fn api_url_np() {
        let r = RawApi::v1NetworkPolicy().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/networking.k8s.io/v1/namespaces/ns/networkpolicies?"
        );
    }
    #[test]
    fn api_url_ingress() {
        let r = RawApi::v1beta1Ingress().within("ns");
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/extensions/v1beta1/namespaces/ns/ingresses?");
    }
    #[test]
    fn api_url_vattach() {
        let r = RawApi::v1VolumeAttachment();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(req.uri(), "/apis/storage.k8s.io/v1/volumeattachments?");
    }
    #[test]
    fn api_url_admission() {
        let r = RawApi::v1beta1ValidatingWebhookConfiguration();
        let req = r.create(&PostParams::default(), vec![]).unwrap();
        assert_eq!(
            req.uri(),
            "/apis/admissionregistration.k8s.io/v1beta1/validatingwebhookconfigurations?"
        );
    }

    #[test]
    fn k8s_obj_custom_ctor() {
        let r = RawApi::v1SelfSubjectRulesReview();
        assert_eq!(r.group, "authorization.k8s.io");
        assert_eq!(r.prefix, "apis");
        assert_eq!(r.resource, "selfsubjectrulesreviews"); // lowercase pluralisation
    }
}
