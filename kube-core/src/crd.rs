//! Traits and tyes for CustomResources

use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions as apiexts;

/// Types for v1 CustomResourceDefinitions
pub mod v1 {
    use super::apiexts::v1::CustomResourceDefinition as Crd;
    /// Extension trait that is implemented by kube-derive
    ///
    /// This trait variant is implemented by default (or when `#[kube(apiextensions = "v1")]`)
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
    pub enum CrdError {
        /// No crds given
        #[error("Empty list of CRDs cannot be merged")]
        MissingCrds,

        /// Served api not present
        #[error("Served api version {0} not found")]
        MissingServedApi(String),

        /// Globals api not present
        #[error("Globals api version {0} not found")]
        MissingGlobalsApi(String),

        /// Globals api not present
        #[error("Given CRD must have exactly one version each")]
        TooManyVersions,

        /// Mismatching api group
        #[error("Mismatching api groups from given CRDs")]
        ApiGroupMismatch,

        /// Mismatching kind
        #[error("Mismatching kinds from given CRDs")]
        KindMismatch,


        //#[error("failed to build request: {0}")]
        //BuildRequest(#[source] http::Error),
        //#[error("failed to serialize body: {0}")]
        //SerializeBody(#[source] serde_json::Error),
    }

    /// Merger for multi-version setups of kube-derived crd schemas
    pub struct CrdMerger {
        crds: Vec<Crd>,
        served: Option<String>,
        globals: Option<String>,
    }

    impl CrdMerger {
        /// Create a CrdMerger from a list of crds
        ///
        /// ```no_run
        /// #let mycrd_v1: CustomResourceDefinition = todo!(); // v1::MyCrd::crd();
        /// #let mycrd_v2: CustomResourceDefinition = todo!(); // v2::MyCrd::crd();
        /// let crds = vec![mycrd_v1, mycrd_v2];
        /// let final_crd = CrdMerger::new(crds).served("v1").merge()?;
        /// ```
        pub fn new(crds: Vec<Crd>) -> Self {
            Self { crds, served: None, globals: None }
        }

        /// Set the apiversion to be served
        pub fn served(mut self, served_apiversion: impl Into<String>) -> Self {
            self.served = Some(served_apiversion.into());
            self
        }

        /// Merge the crds with the given options
        pub fn merge(self) -> Result<Crd, CrdError> { // TODO: error
            if self.crds.is_empty() {
                return Err(CrdError::MissingCrds)
            }
            for crd in self.crds.iter() {
                if crd.spec.versions.len() != 1 {
                    return Err(CrdError::TooManyVersions)
                }
            }
            let mut root = if let Some(g) = self.globals {
                match self.crds.iter().find(|c| c.spec.versions[0].name == g) {
                    None => return Err(CrdError::MissingGlobalsApi(g)),
                    Some(g) => g.clone(),
                }
            } else {
                self.crds.iter().next().unwrap().clone() // we know first is non-empty
            };

            let global_ver = root.spec.versions[0].name.clone();
            let group = &root.spec.group;
            let kind = &root.spec.names.kind;
            // validation
            for crd in self.crds.iter() {
                if &crd.spec.group != group {
                    return Err(CrdError::ApiGroupMismatch)
                }
                if &crd.spec.names.kind != kind {
                    return Err(CrdError::KindMismatch)
                }
                // TODO: validate conversion hooks
            }

            // validation ok, smash them together:
            let versions = &mut root.spec.versions;
            for crd in self.crds {
                if crd.spec.versions[0].name == global_ver {
                    continue;
                }
                versions.push(crd.spec.versions[0].clone());
            }
            Ok(root)
        }
    }
}

/// re-export the current latest version until a newer one is available in cloud providers
pub use v1::CustomResourceExt;
