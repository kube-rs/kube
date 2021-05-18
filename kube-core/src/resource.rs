pub use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;
use once_cell::sync::Lazy;
use std::{borrow::Cow, collections::BTreeMap};

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
    /// See [`DynamicObject`](crate::api::DynamicObject) for a valid implementation of non-k8s-openapi resources.
    type DynamicType: Send + Sync + 'static;

    /// Returns kind of this object
    fn kind(dt: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns group of this object
    fn group(dt: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns version of this object
    fn version(dt: &Self::DynamicType) -> Cow<'_, str>;
    /// Returns apiVersion of this object
    fn api_version(dt: &Self::DynamicType) -> Cow<'_, str> {
        let group = Self::group(dt);
        if group.is_empty() {
            return Self::version(dt);
        }
        let mut group = group.into_owned();
        group.push('/');
        group.push_str(&Self::version(dt));
        group.into()
    }
    /// Returns the plural name of the kind
    ///
    /// This is known as the resource in apimachinery, we rename it for disambiguation.
    /// By default, we infer this name through pluralization.
    ///
    /// The pluralization process is not recommended to be relied upon, and is only used for
    /// `k8s_openapi` types, where we maintain a list of special pluralisations for compatibility.
    ///
    /// Thus when used with `DynamicObject` or `kube-derive`, we override this with correct values.
    fn plural(dt: &Self::DynamicType) -> Cow<'_, str> {
        to_plural(&Self::kind(dt).to_ascii_lowercase()).into()
    }

    /// Creates a url path for http requests for this resource
    fn url_path(dt: &Self::DynamicType, namespace: Option<&str>) -> String {
        let n = if let Some(ns) = namespace {
            format!("namespaces/{}/", ns)
        } else {
            "".into()
        };
        let group = Self::group(dt);
        let api_version = Self::api_version(dt);
        let plural = Self::plural(dt);
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
    /// Metadata that all persisted resources must have
    fn meta_mut(&mut self) -> &mut ObjectMeta;
}

/// Implement accessor trait for any ObjectMeta-using Kubernetes Resource
impl<K> Resource for K
where
    K: k8s_openapi::Metadata<Ty = ObjectMeta>,
{
    type DynamicType = ();

    fn kind(_: &()) -> Cow<'_, str> {
        K::KIND.into()
    }

    fn group(_: &()) -> Cow<'_, str> {
        K::GROUP.into()
    }

    fn version(_: &()) -> Cow<'_, str> {
        K::VERSION.into()
    }

    fn api_version(_: &()) -> Cow<'_, str> {
        K::API_VERSION.into()
    }

    fn meta(&self) -> &ObjectMeta {
        self.metadata()
    }

    fn meta_mut(&mut self) -> &mut ObjectMeta {
        self.metadata_mut()
    }
}

/// Helper methods for resources.
pub trait ResourceExt: Resource {
    /// Returns the name of the resource, panicking if it is
    /// missing. Use this function if you know that name is set, for example
    /// when resource was received from the apiserver.
    /// Because of `.metadata.generateName` field, in other contexts name
    /// may be missing.
    ///
    /// For non-panicking alternative, you can directly read `name` field
    /// on the `self.meta()`.
    fn name(&self) -> String;
    /// The namespace the resource is in
    fn namespace(&self) -> Option<String>;
    /// The resource version
    fn resource_version(&self) -> Option<String>;
    /// Unique ID (if you delete resource and then create a new
    /// resource with the same name, it will have different ID)
    fn uid(&self) -> Option<String>;
    /// Returns resource labels
    fn labels(&self) -> &BTreeMap<String, String>;
    /// Provides mutable access to the labels
    fn labels_mut(&mut self) -> &mut BTreeMap<String, String>;
    /// Returns resource annotations
    fn annotations(&self) -> &BTreeMap<String, String>;
    /// Provider mutable access to the annotations
    fn annotations_mut(&mut self) -> &mut BTreeMap<String, String>;
    /// Returns resource owner references
    fn owner_references(&self) -> &[OwnerReference];
    /// Provides mutable access to the owner references
    fn owner_references_mut(&mut self) -> &mut Vec<OwnerReference>;
    /// Returns resource finalizers
    fn finalizers(&self) -> &[String];
    /// Provides mutable access to the finalizers
    fn finalizers_mut(&mut self) -> &mut Vec<String>;
}

// TODO: replace with ordinary static when BTreeMap::new() is no longer
// const-unstable.
static EMPTY_MAP: Lazy<BTreeMap<String, String>> = Lazy::new(BTreeMap::new);

impl<K: Resource> ResourceExt for K {
    fn name(&self) -> String {
        self.meta().name.clone().expect(".metadata.name missing")
    }

    fn namespace(&self) -> Option<String> {
        self.meta().namespace.clone()
    }

    fn resource_version(&self) -> Option<String> {
        self.meta().resource_version.clone()
    }

    fn uid(&self) -> Option<String> {
        self.meta().uid.clone()
    }

    fn labels(&self) -> &BTreeMap<String, String> {
        self.meta().labels.as_ref().unwrap_or_else(|| &*EMPTY_MAP)
    }

    fn labels_mut(&mut self) -> &mut BTreeMap<String, String> {
        self.meta_mut().labels.get_or_insert_with(BTreeMap::new)
    }

    fn annotations(&self) -> &BTreeMap<String, String> {
        self.meta().annotations.as_ref().unwrap_or_else(|| &*EMPTY_MAP)
    }

    fn annotations_mut(&mut self) -> &mut BTreeMap<String, String> {
        self.meta_mut().annotations.get_or_insert_with(BTreeMap::new)
    }

    fn owner_references(&self) -> &[OwnerReference] {
        self.meta().owner_references.as_deref().unwrap_or_default()
    }

    fn owner_references_mut(&mut self) -> &mut Vec<OwnerReference> {
        self.meta_mut().owner_references.get_or_insert_with(Vec::new)
    }

    fn finalizers(&self) -> &[String] {
        self.meta().finalizers.as_deref().unwrap_or_default()
    }

    fn finalizers_mut(&mut self) -> &mut Vec<String> {
        self.meta_mut().finalizers.get_or_insert_with(Vec::new)
    }
}

// Simple pluralizer. Handles the special cases.
pub(crate) fn to_plural(word: &str) -> String {
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
