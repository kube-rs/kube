//! Simple client for `authentication.kubernetes.io` and `authorization.kubernetes.io` groups
//! See [`AuthClient`] for usage.

use std::collections::BTreeMap;

use crate::{api::Api, client::Client, Result};
use k8s_openapi::api::authentication::v1::{TokenReview, TokenReviewSpec, UserInfo};
use k8s_openapi::api::authorization::v1::{
    NonResourceAttributes, ResourceAttributes, SubjectAccessReview, SubjectAccessReviewSpec,
};
use kube_core::Resource;

/// `AuthClient` can be used to delegate authentication and/or authorization
/// to Kubernetes API server (same way Kubelet or extension API servers may do it).
/// # Permissions
/// `system:auth-delegator` role is enough for all methods to work.
#[derive(Clone)]
pub struct AuthClient {
    token_reviews: Api<TokenReview>,
    subject_access_reviews: Api<SubjectAccessReview>,
}

/// Describes whether token passed all necessary. See [`AuthClient::validate_token`] for details.
#[derive(Debug, Clone)]
pub enum TokenValidity {
    /// Token passed all checks.
    Valid {
        /// User information. This information can be trusted because
        /// it comes from server.
        user: UserInfo,
        /// Audiences token is authorized against. This is subset of audiences provided
        /// on input.
        audiences: Vec<String>,
    },
    /// Token failed some checks and should not be trusted.
    Invalid {
        /// Contains optional error message.
        reason: Option<String>,
    },
    /// Internal error (i.e. validation failed, but it is not caused by token)
    Fail,
}

/// Describes whether operation is authorized. See [`AuthClient::check_access`] for details.
#[derive(Debug, Clone)]
pub enum AccessStatus {
    /// Operation is authorized.
    Allow,
    /// Operation is not authorized.
    Deny {
        /// Contains optional error message.
        reason: Option<String>,
    },
    /// Authorizer has no opinion.
    /// May be treated e.g. as implicit `Deny`.
    NoOpinion,
    /// Internal error
    Fail,
}

impl AuthClient {
    /// Creates new `AuthClient` which will use given `client` for making kubernetes API requests.
    pub fn new(client: Client) -> Self {
        AuthClient {
            token_reviews: Api::all(client.clone()),
            subject_access_reviews: Api::all(client),
        }
    }
    /// Verifies that token is valid and was issued for at least one of provided audiences.
    /// If `audiences` is empty this function will verify that token can be used with cluster API.
    /// ```no_run
    /// # async fn _(client: &AuthClient, token: &str) -> Result<(), Box<dyn std::error::Error>> {
    /// let res = client.validate_token(token, &["my-server"]).await?;
    /// if matches!(res, TokenValidity::Valid) {
    ///     println!("token is valid");
    /// }
    /// # }
    /// ```
    pub async fn validate_token(&self, token: &str, expected_audiences: &[&str]) -> Result<TokenValidity> {
        let token_review_request = TokenReview {
            spec: TokenReviewSpec {
                audiences: Some(expected_audiences.iter().map(ToString::to_string).collect()),
                token: Some(token.to_string()),
            },
            ..Default::default()
        };
        let token_review = self
            .token_reviews
            .create(&Default::default(), &token_review_request)
            .await?;
        let status = match token_review.status {
            Some(s) => s,
            None => return Ok(TokenValidity::Fail),
        };
        if status.authenticated != Some(true) {
            return Ok(TokenValidity::Invalid { reason: status.error });
        }
        // API references recommends additionally validate audiences
        let audiences = status.audiences.as_deref().unwrap_or_default();
        if !expected_audiences.is_empty()
            && !audiences.iter().any(|a| expected_audiences.contains(&a.as_str()))
        {
            return Ok(TokenValidity::Invalid {
                reason: Some("invalid token audiences".to_string()),
            });
        }
        let user = match status.user {
            Some(u) => u,
            None => return Ok(TokenValidity::Fail),
        };
        Ok(TokenValidity::Valid {
            user,
            audiences: audiences.to_vec(),
        })
    }

    /// Verifies that user can do operation.
    /// ```no_run
    /// # use  k8s_openapi::api::core::v1::Pod;
    /// # async fn _(client: &AuthClient, user_info: UserInfo) -> Result<(), Box<dyn std::error::Error>> {
    /// let attrs = ResourceAttributesBuilder::new()
    ///     .set_from_resource::<Pod>()
    ///     .all_verbs()
    ///     .all_objects()
    ///     .build();
    /// let spec = SubjectAccessReviewBuilder::new()
    ///     .set_from_user_info(user_info)
    ///     .resource(attrs)
    ///     .build();
    /// let res = client.check_access(spec).await?;
    /// if matches!(res, AccessStatus::Allow) {
    ///     println!("access granted");
    /// }
    /// # }
    /// ```
    pub async fn check_access(&self, spec: SubjectAccessReviewSpec) -> Result<AccessStatus> {
        let review_request = SubjectAccessReview {
            spec,
            ..Default::default()
        };
        let review = self
            .subject_access_reviews
            .create(&Default::default(), &review_request)
            .await?;
        let status = match review.status {
            Some(s) => s,
            None => return Ok(AccessStatus::Fail),
        };
        if status.allowed {
            return Ok(AccessStatus::Allow);
        }
        if status.denied == Some(true) {
            return Ok(AccessStatus::Deny {
                reason: status.reason,
            });
        }
        Ok(AccessStatus::NoOpinion)
    }
}

/// Helper for creating `SubjectAccessReview`.
/// At first you should specify user information, and then
/// call `resource` or `non_resource` methods, which return specialized
/// builders.
pub struct SubjectAccessReviewBuilder(SubjectAccessReviewSpec);

impl SubjectAccessReviewBuilder {
    /// Creates new builder.
    pub fn new() -> Self {
        SubjectAccessReviewBuilder(SubjectAccessReviewSpec::default())
    }

    /// Set user who makes the request.
    pub fn user(&mut self, user: &str) {
        self.0.user = Some(user.to_string());
    }
    /// Set uid of the user who makes the request.
    pub fn user_uid(&mut self, uid: &str) {
        self.0.uid = Some(uid.to_string());
    }

    /// Set groups of the user who makes the request.
    /// Any existing value will be overwitten.
    pub fn groups(&mut self, groups: &[&str]) {
        self.0.groups = Some(groups.iter().map(ToString::to_string).collect());
    }

    /// Sets extras of the user who makes the request.
    /// Any existing value will be overwitten.
    pub fn extras(&mut self, extras: BTreeMap<String, Vec<String>>) {
        self.0.extra = Some(extras);
    }
    /// Appends extra value. If an extra with the same `name` is already present, `value` is appended.
    pub fn add_extra(&mut self, name: &str, value: &str) {
        let extras = self.0.extra.get_or_insert_with(Default::default);
        let values = extras.entry(name.to_string()).or_default();
        values.push(value.to_string());
    }
    /// Sets all values from the given `UserInfo` value.
    pub fn set_from_user_info(&mut self, info: UserInfo) {
        self.0.uid = info.uid;
        self.0.user = info.username;
        self.0.groups = info.groups;
        self.0.extra = info.extra;
    }

    /// Finalizes builder with given resource attributes and returns SubjectAccessReview spec.
    pub fn resource(mut self, attrs: ResourceAttributes) -> SubjectAccessReviewSpec {
        self.0.resource_attributes = Some(attrs);
        self.0
    }

    /// Finalizes builder with given non-resource attributes and returns SubjectAccessReview spec.
    pub fn non_resource(mut self, attrs: NonResourceAttributes) -> SubjectAccessReviewSpec {
        self.0.non_resource_attributes = Some(attrs);
        self.0
    }
}

/// See [`SubjectAccessReviewBuilder`].
#[derive(Clone)]
pub struct ResourceAttributesBuilder(ResourceAttributes);

impl ResourceAttributesBuilder {
    /// Creates empty builder
    pub fn new() -> Self {
        ResourceAttributesBuilder(Default::default())
    }
    /// Check access for the given api group
    pub fn api_group(mut self, group: &str) -> Self {
        self.0.group = Some(group.to_string());
        self
    }
    /// Check access for all api groups
    pub fn all_api_groups(self) -> Self {
        self.api_group("*")
    }

    /// Check access for the given version
    pub fn version(mut self, version: &str) -> Self {
        self.0.version = Some(version.to_string());
        self
    }
    /// Check access for all versions
    pub fn all_versions(self) -> Self {
        self.version("*")
    }

    /// Check access for the given resource.
    pub fn resource_name(mut self, name: &str) -> Self {
        self.0.resource = Some(name.to_string());
        self
    }
    /// Check access for all resources.
    pub fn all_resources(self) -> Self {
        self.resource_name("*")
    }
    /// Sets api group, version and resource name
    pub fn set_from_resource<K: Resource>(self, dt: &K::DynamicType) -> Self {
        self.api_group(&K::group(dt))
            .version(&K::version(dt))
            .resource_name(&K::plural(dt))
    }

    /// Check access for the given verb.
    /// This function accepts Kubernetes operation verbs, such as `get` or `create`.
    pub fn verb(mut self, verb: &str) -> Self {
        self.0.verb = Some(verb.to_string());
        self
    }

    /// Check access for all operation verbs.
    pub fn all_verbs(self) -> Self {
        self.verb("*")
    }

    /// Check access for the given subresource (instead of the whole resource).
    pub fn subresource(mut self, sr: &str) -> Self {
        self.0.subresource = Some(sr.to_string());
        self
    }

    /// Check access for the object with given name.
    pub fn object_name(mut self, name: &str) -> Self {
        self.0.name = Some(name.to_string());
        self
    }
    /// Check access for all objects.
    pub fn all_objects(self) -> Self {
        self.object_name("")
    }
    /// Check access for the given namespace.
    pub fn namespace(mut self, ns: &str) -> Self {
        self.0.namespace = Some(ns.to_string());
        self
    }
    /// Check access for all namespaces (for namespace-scoped resources)
    /// or in global scope (for non-namespace-scoped resources).
    pub fn global(self) -> Self {
        self.namespace("")
    }

    /// Finalizes builder and returns resource attributes.
    pub fn build(self) -> ResourceAttributes {
        self.0
    }
}

/// See [`SubjectAccessReviewBuilder`].
#[derive(Clone)]
pub struct NonResourceAttributesBuilder(NonResourceAttributes);

impl NonResourceAttributesBuilder {
    /// Creates empty builder
    pub fn new() -> Self {
        NonResourceAttributesBuilder(Default::default())
    }
    /// HTTP request path
    pub fn path(mut self, path: &str) -> Self {
        self.0.path = Some(path.to_string());
        self
    }
    /// HTTP request verb, such as `GET` or `POST`.
    pub fn verb(mut self, verb: &str) -> Self {
        self.0.verb = Some(verb.to_string());
        self
    }
    /// Finalizes builder and returns non-resource attributes.
    pub fn build(self) -> NonResourceAttributes {
        self.0
    }
}
