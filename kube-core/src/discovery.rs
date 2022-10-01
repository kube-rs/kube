//! Type information structs for API discovery
use crate::{gvk::GroupVersionKind, resource::Resource, scope::Scope};
use serde::{Deserialize, Serialize};

/// Information about a Kubernetes API resource
///
/// Used as dynamic type info for `Resource` to allow dynamic querying on `Api`
/// via constructors like `Api::all_with` and `Api::namespaced_with`.
///
/// Only the instances returned by either:
///
/// - `discovery` module in kube/kube-client
/// - `CustomResource` derive in kube-derive
///
/// Will have ALL the extraneous data about shortnames, verbs, and resources.
///
/// # Warning
///
/// Construction through
/// - [`ApiResource::erase`] (type erasing where we have trait data)
/// - [`ApiResource::new`] (proving all essential data manually)
///
/// Are **minimal** conveniences that will work with the Api, but will not have all the extraneous data.
///
/// Shorter construction methods (such as manually filling in data), or fallibly converting from GVKs,
/// may fail to query. Provide accurate `plural` and `namespaced` data to be safe.
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
    pub namespaced: bool,

    /// Supported verbs
    ///
    /// Note: only populated when constructed through discovery or kube-derive
    pub verbs: Vec<String>,

    /// Supported shortnames
    ///
    /// Note: only populated when constructed through discovery or kube-derive.
    pub shortnames: Vec<String>,

    /// Supported subresources
    ///
    /// Note: only populated when constructed through discovery.
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
            namespaced: <K as Resource>::Scope::is_namespaced(),
            // discovery/derive-only properties left blank
            verbs: vec![],
            subresources: vec![],
            shortnames: vec![],
        }
    }

    /// Creates a new ApiResource from a GVK, plural and a namespaced bool
    ///
    /// This is the **minimal** variant needed to use with the dynamic api
    /// It does not contain information abut verbs, subresources and shortnames.
    pub fn new(gvk: &GroupVersionKind, plural: &str, namespaced: bool) -> Self {
        ApiResource {
            api_version: gvk.api_version(),
            group: gvk.group.clone(),
            version: gvk.version.clone(),
            kind: gvk.kind.clone(),
            plural: plural.to_string(),
            namespaced: namespaced,
            // non-essential properties left blank
            verbs: vec![],
            subresources: vec![],
            shortnames: vec![],
        }
    }

    /// Infer a minimal ApiResource from a GVK as cluster scoped
    ///
    /// # Warning
    /// This function will **guess** the resource plural name which can fail
    /// for CRDs with complex pluralisations it can fail. It will also assume cluster scope.
    ///
    /// If you are getting your values from `kube_derive` use the generated method for giving you an [`ApiResource`].
    /// Otherwise consider using [`ApiResource::new`](crate::discovery::ApiResource::new)
    /// to explicitly set the plural and scope, or run api discovery on it via `kube::discovery`.
    pub fn from_gvk(gvk: &GroupVersionKind) -> Self {
        ApiResource::new(gvk, &to_plural(&gvk.kind.to_ascii_lowercase()), false)
    }

    /// Set the whether the resource is namsepace scoped
    pub fn namespaced(mut self, namespaced: bool) -> Self {
        self.namespaced = namespaced;
        self
    }

    /// Set the shortnames
    pub fn shortnames(mut self, shortnames: &[&str]) -> Self {
        self.shortnames = shortnames.iter().map(|x| x.to_string()).collect();
        self
    }

    /// Set the allowed verbs
    pub fn verbs(mut self, verbs: &[&str]) -> Self {
        self.verbs = verbs.iter().map(|x| x.to_string()).collect();
        self
    }

    /// Set the default verbs
    pub fn default_verbs(mut self) -> Self {
        self.verbs = verbs::DEFAULT_VERBS.iter().map(|x| x.to_string()).collect();
        self
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

    /// All the default verbs
    pub const DEFAULT_VERBS: &[&str; 8] =
        &[CREATE, GET, LIST, WATCH, DELETE, DELETE_COLLECTION, UPDATE, PATCH];
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
