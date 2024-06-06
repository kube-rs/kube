//! Contains types for implementing admission controllers.
//!
//! For more information on admission controllers, see:
//! <https://kubernetes.io/docs/reference/access-authn-authz/admission-controllers/>
//! <https://kubernetes.io/blog/2019/03/21/a-guide-to-kubernetes-admission-controllers/>
//! <https://github.com/kubernetes/api/blob/master/admission/v1/types.go>

use crate::{
    dynamic::DynamicObject,
    gvk::{GroupVersionKind, GroupVersionResource},
    metadata::TypeMeta,
    resource::Resource,
    Status,
};

use std::collections::HashMap;

use k8s_openapi::{api::authentication::v1::UserInfo, apimachinery::pkg::runtime::RawExtension};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("failed to serialize patch")]
/// Failed to serialize patch.
pub struct SerializePatchError(#[source] serde_json::Error);

#[derive(Debug, Error)]
#[error("failed to convert AdmissionReview into AdmissionRequest")]
/// Failed to convert `AdmissionReview` into `AdmissionRequest`.
pub struct ConvertAdmissionReviewError;

/// The `kind` field in [`TypeMeta`].
pub const META_KIND: &str = "AdmissionReview";
/// The `api_version` field in [`TypeMeta`] on the v1 version.
pub const META_API_VERSION_V1: &str = "admission.k8s.io/v1";
/// The `api_version` field in [`TypeMeta`] on the v1beta1 version.
pub const META_API_VERSION_V1BETA1: &str = "admission.k8s.io/v1beta1";

/// The top level struct used for Serializing and Deserializing AdmissionReview
/// requests and responses.
///
/// This is both the input type received by admission controllers, and the
/// output type admission controllers should return.
///
/// An admission controller should start by inspecting the [`AdmissionRequest`].
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionReview<T: Resource> {
    /// Contains the API version and type of the request.
    #[serde(flatten)]
    pub types: TypeMeta,
    /// Describes the attributes for the admission request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request: Option<AdmissionRequest<T>>,
    /// Describes the attributes for the admission response.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub response: Option<AdmissionResponse>,
}

impl<T: Resource> TryInto<AdmissionRequest<T>> for AdmissionReview<T> {
    type Error = ConvertAdmissionReviewError;

    fn try_into(self) -> Result<AdmissionRequest<T>, Self::Error> {
        match self.request {
            Some(mut req) => {
                req.types = self.types;
                Ok(req)
            }
            None => Err(ConvertAdmissionReviewError),
        }
    }
}

/// An incoming [`AdmissionReview`] request.
///
/// In an admission controller scenario, this is extracted from an [`AdmissionReview`] via [`TryInto`]
///
/// ```no_run
/// use kube::core::{admission::{AdmissionRequest, AdmissionReview}, DynamicObject};
///
/// // The incoming AdmissionReview received by the controller.
/// let body: AdmissionReview<DynamicObject> = todo!();
/// let req: AdmissionRequest<_> = body.try_into().unwrap();
/// ```
///
/// Based on the contents of the request, an admission controller should construct an
/// [`AdmissionResponse`] using:
///
/// - [`AdmissionResponse::deny`] for illegal/rejected requests
/// - [`AdmissionResponse::invalid`] for malformed requests
/// - [`AdmissionResponse::from`] for the happy path
///
/// then wrap the chosen response in an [`AdmissionReview`] via [`AdmissionResponse::into_review`].
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionRequest<T: Resource> {
    /// Copied from the containing [`AdmissionReview`] and used to specify a
    /// response type and version when constructing an [`AdmissionResponse`].
    #[serde(skip)]
    pub types: TypeMeta,
    /// An identifier for the individual request/response. It allows us to
    /// distinguish instances of requests which are otherwise identical (parallel
    /// requests, requests when earlier requests did not modify, etc). The UID is
    /// meant to track the round trip (request/response) between the KAS and the
    /// webhook, not the user request. It is suitable for correlating log entries
    /// between the webhook and apiserver, for either auditing or debugging.
    pub uid: String,
    /// The fully-qualified type of object being submitted (for example, v1.Pod
    /// or autoscaling.v1.Scale).
    pub kind: GroupVersionKind,
    /// The fully-qualified resource being requested (for example, v1.pods).
    pub resource: GroupVersionResource,
    /// The subresource being requested, if any (for example, "status" or
    /// "scale").
    #[serde(default)]
    pub sub_resource: Option<String>,
    /// The fully-qualified type of the original API request (for example, v1.Pod
    /// or autoscaling.v1.Scale). If this is specified and differs from the value
    /// in "kind", an equivalent match and conversion was performed.
    ///
    /// For example, if deployments can be modified via apps/v1 and apps/v1beta1,
    /// and a webhook registered a rule of `apiGroups:["apps"],
    /// apiVersions:["v1"], resources:["deployments"]` and
    /// `matchPolicy:Equivalent`, an API request to apps/v1beta1 deployments
    /// would be converted and sent to the webhook with `kind: {group:"apps",
    /// version:"v1", kind:"Deployment"}` (matching the rule the webhook
    /// registered for), and `requestKind: {group:"apps", version:"v1beta1",
    /// kind:"Deployment"}` (indicating the kind of the original API request).
    /// See documentation for the "matchPolicy" field in the webhook
    /// configuration type for more details.
    #[serde(default)]
    pub request_kind: Option<GroupVersionKind>,
    /// The fully-qualified resource of the original API request (for example,
    /// v1.pods). If this is specified and differs from the value in "resource",
    /// an equivalent match and conversion was performed.
    ///
    /// For example, if deployments can be modified via apps/v1 and apps/v1beta1,
    /// and a webhook registered a rule of `apiGroups:["apps"],
    /// apiVersions:["v1"], resources: ["deployments"]` and `matchPolicy:
    /// Equivalent`, an API request to apps/v1beta1 deployments would be
    /// converted and sent to the webhook with `resource: {group:"apps",
    /// version:"v1", resource:"deployments"}` (matching the resource the webhook
    /// registered for), and `requestResource: {group:"apps", version:"v1beta1",
    /// resource:"deployments"}` (indicating the resource of the original API
    /// request).
    ///
    /// See documentation for the "matchPolicy" field in the webhook
    /// configuration type.
    #[serde(default)]
    pub request_resource: Option<GroupVersionResource>,
    /// The name of the subresource of the original API request, if any (for
    /// example, "status" or "scale"). If this is specified and differs from the
    /// value in "subResource", an equivalent match and conversion was performed.
    /// See documentation for the "matchPolicy" field in the webhook
    /// configuration type.
    #[serde(default)]
    pub request_sub_resource: Option<String>,
    /// The name of the object as presented in the request. On a CREATE
    /// operation, the client may omit name and rely on the server to generate
    /// the name. If that is the case, this field will contain an empty string.
    #[serde(default)]
    pub name: String,
    /// The namespace associated with the request (if any).
    #[serde(default)]
    pub namespace: Option<String>,
    /// The operation being performed. This may be different than the operation
    /// requested. e.g. a patch can result in either a CREATE or UPDATE
    /// Operation.
    pub operation: Operation,
    /// Information about the requesting user.
    pub user_info: UserInfo,
    /// The object from the incoming request. It's `None` for [`DELETE`](Operation::Delete) operations.
    pub object: Option<T>,
    ///  The existing object. Only populated for DELETE and UPDATE requests.
    pub old_object: Option<T>,
    /// Specifies that modifications will definitely not be persisted for this
    /// request.
    #[serde(default)]
    pub dry_run: bool,
    /// The operation option structure of the operation being performed. e.g.
    /// `meta.k8s.io/v1.DeleteOptions` or `meta.k8s.io/v1.CreateOptions`. This
    /// may be different than the options the caller provided. e.g. for a patch
    /// request the performed [`Operation`] might be a [`CREATE`](Operation::Create), in
    /// which case the Options will a `meta.k8s.io/v1.CreateOptions` even though
    /// the caller provided `meta.k8s.io/v1.PatchOptions`.
    #[serde(default)]
    pub options: Option<RawExtension>,
}

/// The operation specified in an [`AdmissionRequest`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Operation {
    /// An operation that creates a resource.
    Create,
    /// An operation that updates a resource.
    Update,
    /// An operation that deletes a resource.
    Delete,
    /// An operation that connects to a resource.
    Connect,
}

/// An outgoing [`AdmissionReview`] response. Constructed from the corresponding
/// [`AdmissionRequest`].
/// ```no_run
/// use kube::core::{
///     admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
///     DynamicObject,
/// };
///
/// // The incoming AdmissionReview received by the controller.
/// let body: AdmissionReview<DynamicObject> = todo!();
/// let req: AdmissionRequest<_> = body.try_into().unwrap();
///
/// // A normal response with no side effects.
/// let _: AdmissionReview<_> = AdmissionResponse::from(&req).into_review();
///
/// // A response rejecting the admission webhook with a provided reason.
/// let _: AdmissionReview<_> = AdmissionResponse::from(&req)
///     .deny("Some rejection reason.")
///     .into_review();
///
/// use json_patch::{AddOperation, Patch, PatchOperation};
/// use jsonptr::Pointer;
///
/// // A response adding a label to the resource.
/// let _: AdmissionReview<_> = AdmissionResponse::from(&req)
///     .with_patch(Patch(vec![PatchOperation::Add(AddOperation {
///         path: Pointer::new(["metadata","labels","my-label"]),
///         value: serde_json::Value::String("my-value".to_owned()),
///     })]))
///     .unwrap()
///     .into_review();
///
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct AdmissionResponse {
    /// Copied from the corresponding consructing [`AdmissionRequest`].
    #[serde(skip)]
    pub types: TypeMeta,
    /// Identifier for the individual request/response. This must be copied over
    /// from the corresponding AdmissionRequest.
    pub uid: String,
    /// Indicates whether or not the admission request was permitted.
    pub allowed: bool,
    /// Extra details into why an admission request was denied. This field IS NOT
    /// consulted in any way if "Allowed" is "true".
    #[serde(rename = "status")]
    pub result: Status,
    /// The patch body. Currently we only support "JSONPatch" which implements
    /// RFC 6902.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Vec<u8>>,
    /// The type of Patch. Currently we only allow "JSONPatch".
    #[serde(skip_serializing_if = "Option::is_none")]
    patch_type: Option<PatchType>,
    /// An unstructured key value map set by remote admission controller (e.g.
    /// error=image-blacklisted). MutatingAdmissionWebhook and
    /// ValidatingAdmissionWebhook admission controller will prefix the keys with
    /// admission webhook name (e.g.
    /// imagepolicy.example.com/error=image-blacklisted). AuditAnnotations will
    /// be provided by the admission webhook to add additional context to the
    /// audit log for this request.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub audit_annotations: HashMap<String, String>,
    /// A list of warning messages to return to the requesting API client.
    /// Warning messages describe a problem the client making the API request
    /// should correct or be aware of. Limit warnings to 120 characters if
    /// possible. Warnings over 256 characters and large numbers of warnings may
    /// be truncated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}

impl<T: Resource> From<&AdmissionRequest<T>> for AdmissionResponse {
    fn from(req: &AdmissionRequest<T>) -> Self {
        Self {
            types: req.types.clone(),
            uid: req.uid.clone(),
            allowed: true,
            result: Default::default(),
            patch: None,
            patch_type: None,
            audit_annotations: Default::default(),
            warnings: None,
        }
    }
}

impl AdmissionResponse {
    /// Constructs an invalid [`AdmissionResponse`]. It doesn't copy the uid from
    /// the corresponding [`AdmissionRequest`], so should only be used when the
    /// original request cannot be read.
    pub fn invalid<T: ToString>(reason: T) -> Self {
        Self {
            // Since we don't have a request to use for construction, just
            // default to "admission.k8s.io/v1beta1", since it is the most
            // supported and we won't be using any of the new fields.
            types: TypeMeta {
                kind: META_KIND.to_owned(),
                api_version: META_API_VERSION_V1BETA1.to_owned(),
            },
            uid: Default::default(),
            allowed: false,
            result: Status::failure(&reason.to_string(), "InvalidRequest"),
            patch: None,
            patch_type: None,
            audit_annotations: Default::default(),
            warnings: None,
        }
    }

    /// Deny the request with a reason. The reason will be sent to the original caller.
    #[must_use]
    pub fn deny<T: ToString>(mut self, reason: T) -> Self {
        self.allowed = false;
        self.result.message = reason.to_string();
        self
    }

    /// Add JSON patches to the response, modifying the object from the request.
    pub fn with_patch(mut self, patch: json_patch::Patch) -> Result<Self, SerializePatchError> {
        self.patch = Some(serde_json::to_vec(&patch).map_err(SerializePatchError)?);
        self.patch_type = Some(PatchType::JsonPatch);

        Ok(self)
    }

    /// Converts an [`AdmissionResponse`] into a generic [`AdmissionReview`] that
    /// can be used as a webhook response.
    pub fn into_review(self) -> AdmissionReview<DynamicObject> {
        AdmissionReview {
            types: self.types.clone(),
            request: None,
            response: Some(self),
        }
    }
}

/// The type of patch returned in an [`AdmissionResponse`].
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum PatchType {
    /// Specifies the patch body implements JSON Patch under RFC 6902.
    #[serde(rename = "JSONPatch")]
    JsonPatch,
}

#[cfg(test)]
mod test {
    const WEBHOOK_BODY: &str = r#"{"kind":"AdmissionReview","apiVersion":"admission.k8s.io/v1","request":{"uid":"0c9a8d74-9cb7-44dd-b98e-09fd62def2f4","kind":{"group":"","version":"v1","kind":"Pod"},"resource":{"group":"","version":"v1","resource":"pods"},"requestKind":{"group":"","version":"v1","kind":"Pod"},"requestResource":{"group":"","version":"v1","resource":"pods"},"name":"echo-pod","namespace":"colin-coder","operation":"CREATE","userInfo":{"username":"colin@coder.com","groups":["system:authenticated"],"extra":{"iam.gke.io/user-assertion":["REDACTED"],"user-assertion.cloud.google.com":["REDACTED"]}},"object":{"kind":"Pod","apiVersion":"v1","metadata":{"name":"echo-pod","namespace":"colin-coder","creationTimestamp":null,"labels":{"app":"echo-server"},"annotations":{"kubectl.kubernetes.io/last-applied-configuration":"{\"apiVersion\":\"v1\",\"kind\":\"Pod\",\"metadata\":{\"annotations\":{},\"labels\":{\"app\":\"echo-server\"},\"name\":\"echo-pod\",\"namespace\":\"colin-coder\"},\"spec\":{\"containers\":[{\"image\":\"jmalloc/echo-server\",\"name\":\"echo-server\",\"ports\":[{\"containerPort\":8080,\"name\":\"http-port\"}]}]}}\n"},"managedFields":[{"manager":"kubectl","operation":"Update","apiVersion":"v1","time":"2021-03-29T23:02:16Z","fieldsType":"FieldsV1","fieldsV1":{"f:metadata":{"f:annotations":{".":{},"f:kubectl.kubernetes.io/last-applied-configuration":{}},"f:labels":{".":{},"f:app":{}}},"f:spec":{"f:containers":{"k:{\"name\":\"echo-server\"}":{".":{},"f:image":{},"f:imagePullPolicy":{},"f:name":{},"f:ports":{".":{},"k:{\"containerPort\":8080,\"protocol\":\"TCP\"}":{".":{},"f:containerPort":{},"f:name":{},"f:protocol":{}}},"f:resources":{},"f:terminationMessagePath":{},"f:terminationMessagePolicy":{}}},"f:dnsPolicy":{},"f:enableServiceLinks":{},"f:restartPolicy":{},"f:schedulerName":{},"f:securityContext":{},"f:terminationGracePeriodSeconds":{}}}}]},"spec":{"volumes":[{"name":"default-token-rxbqq","secret":{"secretName":"default-token-rxbqq"}}],"containers":[{"name":"echo-server","image":"jmalloc/echo-server","ports":[{"name":"http-port","containerPort":8080,"protocol":"TCP"}],"resources":{},"volumeMounts":[{"name":"default-token-rxbqq","readOnly":true,"mountPath":"/var/run/secrets/kubernetes.io/serviceaccount"}],"terminationMessagePath":"/dev/termination-log","terminationMessagePolicy":"File","imagePullPolicy":"Always"}],"restartPolicy":"Always","terminationGracePeriodSeconds":30,"dnsPolicy":"ClusterFirst","serviceAccountName":"default","serviceAccount":"default","securityContext":{},"schedulerName":"default-scheduler","tolerations":[{"key":"node.kubernetes.io/not-ready","operator":"Exists","effect":"NoExecute","tolerationSeconds":300},{"key":"node.kubernetes.io/unreachable","operator":"Exists","effect":"NoExecute","tolerationSeconds":300}],"priority":0,"enableServiceLinks":true},"status":{}},"oldObject":null,"dryRun":false,"options":{"kind":"CreateOptions","apiVersion":"meta.k8s.io/v1"}}}"#;

    use crate::{
        admission::{AdmissionResponse, AdmissionReview, ConvertAdmissionReviewError},
        DynamicObject,
    };

    #[test]
    fn v1_webhook_unmarshals() {
        serde_json::from_str::<AdmissionReview<DynamicObject>>(WEBHOOK_BODY).unwrap();
    }

    #[test]
    fn version_passes_through() -> Result<(), ConvertAdmissionReviewError> {
        let rev = serde_json::from_str::<AdmissionReview<DynamicObject>>(WEBHOOK_BODY).unwrap();
        let rev_typ = rev.types.clone();
        let res = AdmissionResponse::from(&rev.try_into()?).into_review();

        // Ensure TypeMeta was correctly deserialized.
        assert_ne!(&rev_typ.api_version, "");
        // The TypeMeta should be correctly passed through from the incoming
        // request.
        assert_eq!(&rev_typ, &res.types);
        Ok(())
    }
}
