#![allow(non_snake_case)]
use std::marker::PhantomData;

use crate::{
    api::{subresource::LoggingObject, Api, Object, RawApi, Void},
    client::APIClient,
};
use inflector::string::pluralize::to_plural;

/// Implement a named constructor on Api for a typed kubernetes Object
///
/// This assumes that RawApi::$name exists first
/// legacy, to be removed
macro_rules! ctor {
    ( $name:tt, $object:ty ) => {
        impl Api<$object> {
            pub fn $name(client: APIClient) -> Self {
                Api {
                    api: RawApi::$name(),
                    client,
                    phantom: PhantomData,
                }
            }
        }
    };
}

/// Bind typemeta properties for a k8s_openapi resource to RawApi
///
/// Constructs a RawApi::vxObjectName constructor with correct names, versions
macro_rules! k8s_obj {
    // 4 argument ver
    ( $name:expr, $version:expr, $group:expr, $prefix:expr) => {
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
    // 3 argument version for empty prefix (lots of api::apps stuff has this)
    ( $name:expr, $version:expr, $group:expr) => {
        k8s_obj!($name, $version, $group, "");
    };
}

/// Bind a k8s_openapi resource struct to Api
///
/// Binds Api::vxObjectName to the RawApi
macro_rules! k8s_ctor {
    // using a standard openapi path with consistent Spec and Status suffixed structs
    ( $name:ident, $version:expr, $openapi:path) => {
        paste::item! {
            type [<Obj $name>] = Object<$openapi::[<$version>]::[<$name Spec>], $openapi::[<$version>]::[<$name Status>]>;
            impl Api<[<Obj $name>]> {
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
macro_rules! k8s_custom_ctor {
    // using a non-standard manual Object (for api inconsistencies)
    ( $versioned_name:ident, $obj:ty) => {
        paste::item! {
            impl Api<$obj> {
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
k8s_obj!("Deployment", "v1", "apps", "apis");
k8s_ctor!(Deployment, "v1", k8s_openapi::api::apps);
k8s_obj!("DaemonSet", "v1", "apps", "apis");
k8s_ctor!(DaemonSet, "v1", k8s_openapi::api::apps);
k8s_obj!("ReplicaSet", "v1", "apps", "apis");
k8s_ctor!(ReplicaSet, "v1", k8s_openapi::api::apps);
k8s_obj!("StatefulSet", "v1", "apps", "apis");
k8s_ctor!(StatefulSet, "v1", k8s_openapi::api::apps);


// api::authorization
use k8s_openapi::api::authorization::v1 as v1Auth;
k8s_obj!("SelfSubjectRulesReview", "v1", "authorization.k8s.io", "apis");
k8s_custom_ctor!(v1SelfSubjectRulesReview, Object<v1Auth::SelfSubjectRulesReviewSpec, v1Auth::SubjectRulesReviewStatus>);
#[test]
fn k8s_obj_auth() {
    let r = RawApi::v1SelfSubjectRulesReview();
    assert_eq!(r.group, "authorization.k8s.io");
    assert_eq!(r.prefix, "apis");
    assert_eq!(r.resource, "selfsubjectrulesreviews"); // lowercase pluralisation
}


// api::autoscaling
k8s_obj!("HorizontalPodAutoscaler", "v1", "autoscaling", "apis");
k8s_ctor!(HorizontalPodAutoscaler, "v1", k8s_openapi::api::autoscaling);


// api::core
k8s_obj!("Pod", "v1", "api");
k8s_ctor!(Pod, "v1", k8s_openapi::api::core);
k8s_obj!("Node", "v1", "api");
k8s_ctor!(Node, "v1", k8s_openapi::api::core);
k8s_obj!("Service", "v1", "api");
k8s_ctor!(Service, "v1", k8s_openapi::api::core);
k8s_obj!("Namespace", "v1", "api");
k8s_ctor!(Namespace, "v1", k8s_openapi::api::core);
k8s_obj!("PersistentVolume", "v1", "api");
k8s_ctor!(PersistentVolume, "v1", k8s_openapi::api::core);
k8s_obj!("ResourceQuota", "v1", "api");
k8s_ctor!(ResourceQuota, "v1", k8s_openapi::api::core);

k8s_obj!("PersistentVolumeClaim", "v1", "api");
k8s_ctor!(PersistentVolumeClaim, "v1", k8s_openapi::api::core);

k8s_obj!("ReplicationController", "v1", "api");
k8s_ctor!(ReplicationController, "v1", k8s_openapi::api::core);

use k8s_openapi::api::core::v1 as v1Core;
impl LoggingObject for Object<v1Core::PodSpec, v1Core::PodStatus> {}
impl LoggingObject for Object<v1Core::PodSpec, Void> {}


// api::batch
k8s_obj!("CronJob", "v1beta1", "batch", "apis");
k8s_ctor!(CronJob, "v1beta1", k8s_openapi::api::batch);
k8s_obj!("Job", "v1", "batch", "apis");
k8s_ctor!(Job, "v1", k8s_openapi::api::batch);


// api::extensions
k8s_obj!("Ingress", "v1beta1", "extensions", "apis");
k8s_ctor!(Ingress, "v1beta1", k8s_openapi::api::extensions);

// apiextensions_apiserver::pkg::apis::apiextensions
k8s_obj!("CustomResourceDefinition", "v1beta1", "apiextensions.k8s.io", "apis");
k8s_ctor!(CustomResourceDefinition, "v1beta1", k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions);

// api::storage::v1
use k8s_openapi::api::storage::v1::{VolumeAttachmentSpec, VolumeAttachmentStatus};
ctor!(v1VolumeAttachment, Object<VolumeAttachmentSpec, VolumeAttachmentStatus>);

// api::networking::v1
use k8s_openapi::api::networking::v1::NetworkPolicySpec;
ctor!(v1NetworkPolicy, Object<NetworkPolicySpec, Void>); // has no Status
