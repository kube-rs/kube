//! Indirection layer for generated structs for easier imports

/// Re-export k8s-openapi root modules
pub use k8s_openapi::{api, apiextensions_apiserver, apimachinery};
/// Re-export versioned stable modules as the client-go like equivalent names
///
/// Names should generally match https://pkg.go.dev/k8s.io/client-go/kubernetes/typed
#[rustfmt::skip]
pub use {
    api::admissionregistration::v1 as admissionregistrationv1,
    api::apps::v1 as appsv1,
    api::authentication::v1 as authenticationv1,
    api::authorization::v1 as authorizationv1,
    api::autoscaling::v1 as autoscalingv1,
    api::autoscaling::v2 as autoscalingv2,
    api::batch::v1 as batchv1,
    api::certificates::v1 as certificatesv1,
    api::coordination::v1 as coordinationv1,
    api::core::v1 as corev1,
    api::discovery::v1 as discoveryv1,
    api::events::v1 as eventsv1,
    api::networking::v1 as networkingv1,
    api::node::v1 as nodev1,
    api::policy::v1 as policyv1,
    api::rbac::v1 as rbacv1,
    api::scheduling::v1 as schedulingv1,
    api::storage::v1 as storagev1,
    apiextensions_apiserver::pkg::apis::apiextensions::v1 as apiextensionsv1,
    apimachinery::pkg::apis::meta::v1 as metav1,
};

// Names with version gates
k8s_openapi::k8s_if_ge_1_26! {
    pub use api::flowcontrol::v1 as flowcontrolv1;
}
