//! Traits and tyes for CustomResources

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions as apiexts;

/// Types for v1 CustomResourceDefinitions
pub mod v1 {
    use super::apiexts::v1::CustomResourceDefinition as Crd;
    /// Extension trait that is implemented by kube-derive
    pub trait CustomResourceExt {
        /// Helper to generate the CRD including the JsonSchema
        ///
        /// This is using the stable v1::CustomResourceDefinitions (present in kubernetes >= 1.16)
        fn crd() -> Crd;
        /// Helper to return the name of this `CustomResourceDefinition` in kubernetes.
        ///
        /// This is not the name of an _instance_ of this custom resource but the `CustomResourceDefinition` object itself.
        fn crd_name() -> &'static str;
        /// Helper to generate the api information type for use with the dynamic `Api`
        fn api_resource() -> crate::discovery::ApiResource;
        /// Shortnames of this resource type.
        ///
        /// For example: [`Pod`] has the shortname alias `po`.
        ///
        /// NOTE: This function returns *declared* short names (at compile-time, using the `#[kube(shortname = "foo")]`), not the
        /// shortnames registered with the Kubernetes API (which is what tools such as `kubectl` look at).
        ///
        /// [`Pod`]: `k8s_openapi::api::core::v1::Pod`
        fn shortnames() -> &'static [&'static str];
    }

    /// Possible errors when merging CRDs
    #[derive(Debug, thiserror::Error)]
    pub enum MergeError {
        /// No crds given
        #[error("empty list of CRDs cannot be merged")]
        MissingCrds,

        /// Stored api not present
        #[error("stored api version {0} not found")]
        MissingStoredApi(String),

        /// Root api not present
        #[error("root api version {0} not found")]
        MissingRootVersion(String),

        /// No versions given in one crd to merge
        #[error("given CRD must have versions")]
        MissingVersions,

        /// Too many versions given to individual crds
        #[error("mergeable CRDs cannot have multiple versions")]
        MultiVersionCrd,

        /// Mismatching spec properties on crds
        #[error("mismatching {0} property from given CRDs")]
        PropertyMismatch(String),
    }

    /// Merge a collection of crds into a single multiversion crd
    ///
    /// Given multiple [`CustomResource`] derived types granting [`CRD`]s via [`CustomResourceExt::crd`],
    /// we can merge them into a single [`CRD`] with multiple [`CRDVersion`] objects, marking only
    /// the specified apiversion as `storage: true`.
    ///
    /// This merge algorithm assumes that every [`CRD`]:
    ///
    /// - exposes exactly one [`CRDVersion`]
    /// - uses identical values for `spec.group`, `spec.scope`, and `spec.names.kind`
    ///
    /// This is always true for [`CustomResource`] derives.
    ///
    /// ## Usage
    ///
    /// ```no_run
    /// # use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
    /// use kube::core::crd::merge_crds;
    /// # let mycrd_v1: CustomResourceDefinition = todo!(); // v1::MyCrd::crd();
    /// # let mycrd_v2: CustomResourceDefinition = todo!(); // v2::MyCrd::crd();
    /// let crds = vec![mycrd_v1, mycrd_v2];
    /// let multi_version_crd = merge_crds(crds, "v1").unwrap();
    /// ```
    ///
    /// Note the merge is done by marking the:
    ///
    /// - crd containing the `stored_apiversion` as the place the other crds merge their [`CRDVersion`] items
    /// - stored version is marked with `storage: true`, while all others get `storage: false`
    ///
    /// [`CustomResourceExt::crd`]: crate::CustomResourceExt::crd
    /// [`CRD`]: https://docs.rs/k8s-openapi/latest/k8s_openapi/apiextensions_apiserver/pkg/apis/apiextensions/v1/struct.CustomResourceDefinition.html
    /// [`CRDVersion`]: https://docs.rs/k8s-openapi/latest/k8s_openapi/apiextensions_apiserver/pkg/apis/apiextensions/v1/struct.CustomResourceDefinitionVersion.html
    /// [`CustomResource`]: https://docs.rs/kube/latest/kube/derive.CustomResource.html
    pub fn merge_crds(mut crds: Vec<Crd>, stored_apiversion: &str) -> Result<Crd, MergeError> {
        if crds.is_empty() {
            return Err(MergeError::MissingCrds);
        }
        for crd in crds.iter() {
            if crd.spec.versions.is_empty() {
                return Err(MergeError::MissingVersions);
            }
            if crd.spec.versions.len() != 1 {
                return Err(MergeError::MultiVersionCrd);
            }
        }
        let ver = stored_apiversion;
        let found = crds.iter().position(|c| c.spec.versions[0].name == ver);
        // Extract the root/first object to start with (the one we will merge into)
        let mut root = match found {
            None => return Err(MergeError::MissingRootVersion(ver.into())),
            Some(idx) => crds.remove(idx),
        };
        root.spec.versions[0].storage = true; // main version - set true in case modified

        // Values that needs to be identical across crds:
        let group = &root.spec.group;
        let kind = &root.spec.names.kind;
        let scope = &root.spec.scope;
        // sanity; don't merge crds with mismatching groups, versions, or other core properties
        for crd in crds.iter() {
            if &crd.spec.group != group {
                return Err(MergeError::PropertyMismatch("group".to_string()));
            }
            if &crd.spec.names.kind != kind {
                return Err(MergeError::PropertyMismatch("kind".to_string()));
            }
            if &crd.spec.scope != scope {
                return Err(MergeError::PropertyMismatch("scope".to_string()));
            }
        }

        // combine all version objects into the root object
        let versions = &mut root.spec.versions;
        while let Some(mut crd) = crds.pop() {
            while let Some(mut v) = crd.spec.versions.pop() {
                v.storage = false; // secondary versions
                versions.push(v);
            }
        }
        Ok(root)
    }

    mod tests {
        #[test]
        fn crd_merge() {
            use super::{merge_crds, Crd};
            let crd1 = r#"
            apiVersion: apiextensions.k8s.io/v1
            kind: CustomResourceDefinition
            metadata:
              name: multiversions.kube.rs
            spec:
              group: kube.rs
              names:
                categories: []
                kind: MultiVersion
                plural: multiversions
                shortNames: []
                singular: multiversion
              scope: Namespaced
              versions:
              - additionalPrinterColumns: []
                name: v1
                schema:
                  openAPIV3Schema:
                    type: object
                    x-kubernetes-preserve-unknown-fields: true
                served: true
                storage: true"#;

            let crd2 = r#"
            apiVersion: apiextensions.k8s.io/v1
            kind: CustomResourceDefinition
            metadata:
              name: multiversions.kube.rs
            spec:
              group: kube.rs
              names:
                categories: []
                kind: MultiVersion
                plural: multiversions
                shortNames: []
                singular: multiversion
              scope: Namespaced
              versions:
              - additionalPrinterColumns: []
                name: v2
                schema:
                  openAPIV3Schema:
                    type: object
                    x-kubernetes-preserve-unknown-fields: true
                served: true
                storage: true"#;

            let expected = r#"
            apiVersion: apiextensions.k8s.io/v1
            kind: CustomResourceDefinition
            metadata:
              name: multiversions.kube.rs
            spec:
              group: kube.rs
              names:
                categories: []
                kind: MultiVersion
                plural: multiversions
                shortNames: []
                singular: multiversion
              scope: Namespaced
              versions:
              - additionalPrinterColumns: []
                name: v2
                schema:
                  openAPIV3Schema:
                    type: object
                    x-kubernetes-preserve-unknown-fields: true
                served: true
                storage: true
              - additionalPrinterColumns: []
                name: v1
                schema:
                  openAPIV3Schema:
                    type: object
                    x-kubernetes-preserve-unknown-fields: true
                served: true
                storage: false"#;


            let c1: Crd = serde_yaml::from_str(crd1).unwrap();
            let c2: Crd = serde_yaml::from_str(crd2).unwrap();
            let ce: Crd = serde_yaml::from_str(expected).unwrap();
            let combined = merge_crds(vec![c1, c2], "v2").unwrap();

            let combo_json = serde_json::to_value(&combined).unwrap();
            let exp_json = serde_json::to_value(&ce).unwrap();
            assert_json_diff::assert_json_eq!(combo_json, exp_json);
        }
    }
}

// re-export current latest (v1)
pub use v1::{merge_crds, CustomResourceExt, MergeError};
