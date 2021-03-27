pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::{ListMeta, ObjectMeta};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

/// An accessor trait for a kubernetes Resource.
///
/// This is for a subset of Kubernetes type that do not end in `List`.
/// These types, using [`ObjectMeta`], SHOULD all have required properties:
/// - `.metadata`
/// - `.metadata.name`
///
/// And these optional properties:
/// - `.metadata.namespace`
/// - `.metadata.resource_version`
///
/// This avoids a bunch of the unnecessary unwrap mechanics for apps.
pub trait Resource {
    /// Type information for types that do not know their resource information at compile time.
    ///
    /// Types that know their metadata at compile time should select `DynamicType = ()`.
    /// Types that require some information at runtime should select `DynamicType`
    /// as type of this information.
    ///
    /// See [`DynamicObject`] for a valid implementation of non-k8s-openapi resources.
    type DynamicType: Send + Sync + 'static;

    /// Returns kind of this object
    fn kind(f: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns group of this object
    fn group(f: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns version of this object
    fn version(f: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns apiVersion of this object
    fn api_version(f: &Self::DynamicType) -> Cow<'_, str> {
        let group = Self::group(f);
        if group.is_empty() {
            return Self::version(f);
        }
        let mut group = group.into_owned();
        group.push('/');
        group.push_str(&Self::version(f));
        group.into()
    }

    /// Creates a url path for http requests for this resource
    fn url_path(t: &Self::DynamicType, namespace: Option<&str>) -> String {
        let n = if let Some(ns) = namespace {
            format!("namespaces/{}/", ns)
        } else {
            "".into()
        };
        let group = Self::group(t);
        let api_version = Self::api_version(t);
        let plural = to_plural(&Self::kind(t).to_ascii_lowercase());
        format!(
            "/{group}/{api_version}/{namespaces}{plural}",
            group = if group.is_empty() { "api" } else { "apis" },
            api_version = api_version,
            namespaces = n,
            plural = plural
        )
    }

    /// Metadata that all persisted resources must have
    fn meta(&self) -> &ObjectMeta;
    /// The name of the resource
    fn name(&self) -> String;
    /// The namespace the resource is in
    fn namespace(&self) -> Option<String>;
    /// The resource version
    fn resource_ver(&self) -> Option<String>;
}

/// Implement accessor trait for any ObjectMeta-using Kubernetes Resource
impl<K> Resource for K
where
    K: k8s_openapi::Metadata<Ty = ObjectMeta>,
{
    type DynamicType = ();

    fn kind<'a>(_: &()) -> Cow<'_, str> {
        K::KIND.into()
    }

    fn group<'a>(_: &()) -> Cow<'_, str> {
        K::GROUP.into()
    }

    fn version<'a>(_: &()) -> Cow<'_, str> {
        K::VERSION.into()
    }

    fn api_version(_: &()) -> Cow<'_, str> {
        K::API_VERSION.into()
    }

    fn meta(&self) -> &ObjectMeta {
        self.metadata()
    }

    fn name(&self) -> String {
        self.meta().name.clone().expect("kind has metadata.name")
    }

    fn resource_ver(&self) -> Option<String> {
        self.meta().resource_version.clone()
    }

    fn namespace(&self) -> Option<String> {
        self.meta().namespace.clone()
    }
}

/// A convenience struct for ad-hoc serialization.
///
/// Mostly useful for [`Object`](super::Object).
#[derive(Deserialize, Serialize, Clone, Default, Debug, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TypeMeta {
    /// The version of the API
    pub api_version: String,

    /// The name of the API
    pub kind: String,
}

// Simple pluralizer. Handles the special cases.
fn to_plural(word: &str) -> String {
    if word == "endpoints" || word == "endpointslices" {
        return word.to_owned();
    } else if word == "nodemetrics" {
        return "nodes".to_owned();
    } else if word == "podmetrics" {
        return "pods".to_owned();
    }

    // Words ending in s, x, z, ch, sh will be pluralized with -es (eg. foxes).
    if word.ends_with('s')
        || word.ends_with('x')
        || word.ends_with('z')
        || word.ends_with("ch")
        || word.ends_with("sh")
    {
        return format!("{}es", word);
    }

    // Words ending in y that are preceded by a consonant will be pluralized by
    // replacing y with -ies (eg. puppies).
    if word.ends_with('y') {
        if let Some(c) = word.chars().nth(word.len() - 2) {
            if !matches!(c, 'a' | 'e' | 'i' | 'o' | 'u') {
                // Remove 'y' and add `ies`
                let mut chars = word.chars();
                chars.next_back();
                return format!("{}ies", chars.as_str());
            }
        }
    }

    // All other words will have "s" added to the end (eg. days).
    format!("{}s", word)
}

#[test]
fn test_to_plural_native() {
    // Extracted from `swagger.json`
    #[rustfmt::skip]
    let native_kinds = vec![
        ("APIService", "apiservices"),
        ("Binding", "bindings"),
        ("CertificateSigningRequest", "certificatesigningrequests"),
        ("ClusterRole", "clusterroles"), ("ClusterRoleBinding", "clusterrolebindings"),
        ("ComponentStatus", "componentstatuses"),
        ("ConfigMap", "configmaps"),
        ("ControllerRevision", "controllerrevisions"),
        ("CronJob", "cronjobs"),
        ("CSIDriver", "csidrivers"), ("CSINode", "csinodes"), ("CSIStorageCapacity", "csistoragecapacities"),
        ("CustomResourceDefinition", "customresourcedefinitions"),
        ("DaemonSet", "daemonsets"),
        ("Deployment", "deployments"),
        ("Endpoints", "endpoints"), ("EndpointSlice", "endpointslices"),
        ("Event", "events"),
        ("FlowSchema", "flowschemas"),
        ("HorizontalPodAutoscaler", "horizontalpodautoscalers"),
        ("Ingress", "ingresses"), ("IngressClass", "ingressclasses"),
        ("Job", "jobs"),
        ("Lease", "leases"),
        ("LimitRange", "limitranges"),
        ("LocalSubjectAccessReview", "localsubjectaccessreviews"),
        ("MutatingWebhookConfiguration", "mutatingwebhookconfigurations"),
        ("Namespace", "namespaces"),
        ("NetworkPolicy", "networkpolicies"),
        ("Node", "nodes"),
        ("PersistentVolumeClaim", "persistentvolumeclaims"),
        ("PersistentVolume", "persistentvolumes"),
        ("PodDisruptionBudget", "poddisruptionbudgets"),
        ("Pod", "pods"),
        ("PodSecurityPolicy", "podsecuritypolicies"),
        ("PodTemplate", "podtemplates"),
        ("PriorityClass", "priorityclasses"),
        ("PriorityLevelConfiguration", "prioritylevelconfigurations"),
        ("ReplicaSet", "replicasets"),
        ("ReplicationController", "replicationcontrollers"),
        ("ResourceQuota", "resourcequotas"),
        ("Role", "roles"), ("RoleBinding", "rolebindings"),
        ("RuntimeClass", "runtimeclasses"),
        ("Secret", "secrets"),
        ("SelfSubjectAccessReview", "selfsubjectaccessreviews"),
        ("SelfSubjectRulesReview", "selfsubjectrulesreviews"),
        ("ServiceAccount", "serviceaccounts"),
        ("Service", "services"),
        ("StatefulSet", "statefulsets"),
        ("StorageClass", "storageclasses"), ("StorageVersion", "storageversions"),
        ("SubjectAccessReview", "subjectaccessreviews"),
        ("TokenReview", "tokenreviews"),
        ("ValidatingWebhookConfiguration", "validatingwebhookconfigurations"),
        ("VolumeAttachment", "volumeattachments"),
    ];
    for (kind, plural) in native_kinds {
        assert_eq!(to_plural(&kind.to_ascii_lowercase()), plural);
    }
}
