//! Type information structs for API discovery
use crate::{gvk::GroupVersionKind, resource::Resource};
use serde::{Deserialize, Serialize};

/// Information about a Kubernetes API resource
///
/// Enough information to use it like a `Resource` by passing it to the dynamic `Api`
/// constructors like `Api::all_with` and `Api::namespaced_with`.
///
/// Note that this can be constructed in many ways, and all information
/// is only guaranteed to be present through discovery.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ApiResource {
    /// Resource group, empty for core group.
    pub group: String,
    /// group version
    pub version: String,
    /// apiVersion of the resource (v1 for core group,
    /// groupName/groupVersions for other).
    pub api_version: String,
    /// Singular PascalCase name of the resource
    pub kind: String,

    /// Resource name / plural name
    pub plural: String,

    /// Whether the resource is namespaced or not
    ///
    /// Note: only populated through kube-derive and discovery.
    pub namespaced: bool,

    /// Supported verbs
    ///
    /// Note: only populated when constructed through discovery.
    pub verbs: Vec<String>,

    /// Supported subresources
    ///
    /// Note: only populated when constructed through discovery.
    /// TODO: populate through kube-derive
    pub subresources: Vec<ApiResource>,
}


impl ApiResource {
    /// Creates an ApiResource by type-erasing a Resource
    ///
    /// Note that this variant of constructing an `ApiResource` does not
    /// get you verbs and available subresources.
    /// If you need this, construct via discovery.
    pub fn erase<K: Resource>(dt: &K::DynamicType) -> Self {
        ApiResource {
            group: K::group(dt).to_string(),
            version: K::version(dt).to_string(),
            api_version: K::api_version(dt).to_string(),
            kind: K::kind(dt).to_string(),
            plural: K::plural(dt).to_string(),
            verbs: vec![],
            namespaced: false,
            subresources: vec![],
        }
    }

    /// Creates an ApiResource from group, version, kind and plural name.
    pub fn from_gvk_with_plural(gvk: &GroupVersionKind, plural: &str) -> Self {
        ApiResource {
            api_version: gvk.api_version(),
            group: gvk.group.clone(),
            version: gvk.version.clone(),
            kind: gvk.kind.clone(),
            plural: plural.to_string(),
            verbs: vec![],
            namespaced: false,
            subresources: vec![],
        }
    }

    /// Creates an ApiResource from group, version and kind.
    ///
    /// # Warning
    /// This function will **guess** the resource plural name.
    /// Usually, this is ok, but for CRDs with complex pluralisations it can fail.
    /// If you are getting your values from `kube_derive` use the generated method for giving you an [`ApiResource`].
    /// Otherwise consider using [`ApiResource::from_gvk_with_plural`](crate::discovery::ApiResource::from_gvk_with_plural)
    /// to explicitly set the plural, or run api discovery on it via `kube::discovery`.
    pub fn from_gvk(gvk: &GroupVersionKind) -> Self {
        ApiResource::from_gvk_with_plural(gvk, &to_plural(&gvk.kind.to_ascii_lowercase()))
    }
}

/// Rbac verbs
pub mod verbs {
    /// Create a resource
    pub const CREATE: &str = "create";
    /// Get single resource
    pub const GET: &str = "get";
    /// List objects
    pub const LIST: &str = "list";
    /// Watch for objects changes
    pub const WATCH: &str = "watch";
    /// Delete single object
    pub const DELETE: &str = "delete";
    /// Delete multiple objects at once
    pub const DELETE_COLLECTION: &str = "deletecollection";
    /// Update an object
    pub const UPDATE: &str = "update";
    /// Patch an object
    pub const PATCH: &str = "patch";
}

impl ApiResource {
    /// Checks that given verb is supported on this resource.
    pub fn supports_operation(&self, operation: &str) -> bool {
        self.verbs.iter().any(|op| op == operation)
    }
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
